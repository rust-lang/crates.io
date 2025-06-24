use crate::app::AppState;
use crate::email::{Email, EmailMessage};
use crate::models::{ApiToken, User};
use crate::schema::{api_tokens, crate_owners, crates, emails};
use crate::util::errors::{AppResult, BoxedAppError, bad_request};
use crate::util::token::HashedToken;
use anyhow::{Context, anyhow};
use axum::Json;
use axum::body::Bytes;
use base64::{Engine, engine::general_purpose};
use crates_io_database::models::OwnerKind;
use crates_io_database::schema::trustpub_tokens;
use crates_io_github::GitHubPublicKey;
use crates_io_trustpub::access_token::AccessToken;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use futures_util::TryStreamExt;
use http::HeaderMap;
use minijinja::context;
use p256::PublicKey;
use p256::ecdsa::VerifyingKey;
use p256::ecdsa::signature::Verifier;
use serde_json as json;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::str::FromStr;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::warn;

// Minimum number of seconds to wait before refreshing cache of GitHub's public keys
const PUBLIC_KEY_CACHE_LIFETIME: Duration = Duration::from_secs(60 * 60 * 24); // 24 hours

// Cache of public keys that have been fetched from GitHub API
static PUBLIC_KEY_CACHE: LazyLock<Mutex<GitHubPublicKeyCache>> = LazyLock::new(|| {
    let keys: Vec<GitHubPublicKey> = Vec::new();
    let cache = GitHubPublicKeyCache {
        keys,
        timestamp: None,
    };
    Mutex::new(cache)
});

#[derive(Debug, Clone)]
struct GitHubPublicKeyCache {
    keys: Vec<GitHubPublicKey>,
    timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

/// Check if cache of public keys is populated and not expired
fn is_cache_valid(timestamp: Option<chrono::DateTime<chrono::Utc>>) -> bool {
    timestamp.is_some_and(|timestamp| chrono::Utc::now() < timestamp + PUBLIC_KEY_CACHE_LIFETIME)
}

// Fetches list of public keys from GitHub API
async fn get_public_keys(state: &AppState) -> Result<Vec<GitHubPublicKey>, BoxedAppError> {
    // Return list from cache if populated and still valid
    let mut cache = PUBLIC_KEY_CACHE.lock().await;
    if is_cache_valid(cache.timestamp) {
        return Ok(cache.keys.clone());
    }

    // Fetch from GitHub API
    let client_id = &state.config.gh_client_id;
    let client_secret = state.config.gh_client_secret.secret();
    let keys = state.github.public_keys(client_id, client_secret).await?;

    // Populate cache
    cache.keys.clone_from(&keys);
    cache.timestamp = Some(chrono::Utc::now());

    Ok(keys)
}

/// Verifies that the GitHub signature in request headers is valid
async fn verify_github_signature(
    headers: &HeaderMap,
    state: &AppState,
    json: &[u8],
) -> Result<(), BoxedAppError> {
    // Read and decode request headers
    let req_key_id = headers
        .get("GITHUB-PUBLIC-KEY-IDENTIFIER")
        .ok_or_else(|| bad_request("missing HTTP header: GITHUB-PUBLIC-KEY-IDENTIFIER"))?
        .to_str()
        .map_err(|e| bad_request(format!("failed to decode HTTP header: {e:?}")))?;

    let sig = headers
        .get("GITHUB-PUBLIC-KEY-SIGNATURE")
        .ok_or_else(|| bad_request("missing HTTP header: GITHUB-PUBLIC-KEY-SIGNATURE"))?;
    let sig = general_purpose::STANDARD
        .decode(sig)
        .map_err(|e| bad_request(format!("failed to decode signature as base64: {e:?}")))?;
    let sig = p256::ecdsa::Signature::from_der(&sig)
        .map_err(|e| bad_request(format!("failed to parse signature from ASN.1 DER: {e:?}")))?;

    let public_keys = get_public_keys(state)
        .await
        .map_err(|e| bad_request(format!("failed to fetch GitHub public keys: {e:?}")))?;

    let key = public_keys
        .iter()
        .find(|key| key.key_identifier == req_key_id);

    let Some(key) = key else {
        return Err(bad_request(format!("unknown key id {req_key_id}")));
    };

    if !key.is_current {
        let error = bad_request(format!("key id {req_key_id} is not a current key"));
        return Err(error);
    }

    let public_key =
        PublicKey::from_str(&key.key).map_err(|_| bad_request("cannot parse public key"))?;

    VerifyingKey::from(public_key)
        .verify(json, &sig)
        .map_err(|e| bad_request(format!("invalid signature: {e:?}")))?;

    debug!(
        key_id = %key.key_identifier,
        "GitHub secret alert request validated",
    );
    Ok(())
}

#[derive(Deserialize, Serialize)]
struct GitHubSecretAlert {
    token: String,
    r#type: String,
    url: String,
    source: String,
}

/// Revokes an API token or Trusted Publishing token and notifies the token owner
async fn alert_revoke_token(
    state: &AppState,
    alert: &GitHubSecretAlert,
    conn: &mut AsyncPgConnection,
) -> QueryResult<GitHubSecretAlertFeedbackLabel> {
    // First, try to handle as a Trusted Publishing token
    if let Ok(token) = alert.token.parse::<AccessToken>() {
        let hashed_token = token.sha256();

        // Delete the token and return crate_ids for notifications
        let crate_ids = diesel::delete(trustpub_tokens::table)
            .filter(trustpub_tokens::hashed_token.eq(hashed_token.as_slice()))
            .returning(trustpub_tokens::crate_ids)
            .get_result::<Vec<Option<i32>>>(conn)
            .await
            .optional()?;

        let Some(crate_ids) = crate_ids else {
            debug!("Unknown Trusted Publishing token received (false positive)");
            return Ok(GitHubSecretAlertFeedbackLabel::FalsePositive);
        };

        warn!("Active Trusted Publishing token received and revoked (true positive)");

        // Send notification emails to all affected crate owners
        let actual_crate_ids: Vec<i32> = crate_ids.into_iter().flatten().collect();
        let result = send_trustpub_notification_emails(&actual_crate_ids, alert, state, conn).await;
        if let Err(error) = result {
            warn!(
                "Failed to send trusted publishing token exposure notifications for crates {actual_crate_ids:?}: {error}",
            );
        }

        return Ok(GitHubSecretAlertFeedbackLabel::TruePositive);
    }

    // If not a Trusted Publishing token or not found, try as a regular API token
    let hashed_token = HashedToken::hash(&alert.token);

    // Not using `ApiToken::find_by_api_token()` in order to preserve `last_used_at`
    let token = api_tokens::table
        .select(ApiToken::as_select())
        .filter(api_tokens::token.eq(hashed_token))
        .get_result::<ApiToken>(conn)
        .await
        .optional()?;

    let Some(token) = token else {
        debug!("Unknown token received (false positive)");
        return Ok(GitHubSecretAlertFeedbackLabel::FalsePositive);
    };

    if token.revoked {
        debug!(
            token_id = %token.id, user_id = %token.user_id,
            "Already revoked API token received (true positive)",
        );
        return Ok(GitHubSecretAlertFeedbackLabel::TruePositive);
    }

    diesel::update(&token)
        .set(api_tokens::revoked.eq(true))
        .execute(conn)
        .await?;

    warn!(
        token_id = %token.id, user_id = %token.user_id,
        "Active API token received and revoked (true positive)",
    );

    if let Err(error) = send_notification_email(&token, alert, state, conn).await {
        warn!(
            token_id = %token.id, user_id = %token.user_id, ?error,
            "Failed to send email notification",
        )
    }

    Ok(GitHubSecretAlertFeedbackLabel::TruePositive)
}

async fn send_notification_email(
    token: &ApiToken,
    alert: &GitHubSecretAlert,
    state: &AppState,
    conn: &mut AsyncPgConnection,
) -> anyhow::Result<()> {
    let user = User::find(conn, token.user_id)
        .await
        .context("Failed to find user")?;

    let Some(recipient) = user.email(conn).await? else {
        return Err(anyhow!("No address found"));
    };

    let email = EmailMessage::from_template(
        "token_exposed",
        context! {
            domain => state.config.domain_name,
            reporter => "GitHub",
            source => alert.source,
            token_name => token.name,
            url => if alert.url.is_empty() { "" } else { &alert.url }
        },
    )?;

    state.emails.send(&recipient, email).await?;

    Ok(())
}

async fn send_trustpub_notification_emails(
    crate_ids: &[i32],
    alert: &GitHubSecretAlert,
    state: &AppState,
    conn: &mut AsyncPgConnection,
) -> anyhow::Result<()> {
    // Build a mapping from crate_id to crate_name directly from the query
    let crate_id_to_name: HashMap<i32, String> = crates::table
        .select((crates::id, crates::name))
        .filter(crates::id.eq_any(crate_ids))
        .load_stream::<(i32, String)>(conn)
        .await?
        .try_fold(HashMap::new(), |mut map, (id, name)| {
            map.insert(id, name);
            std::future::ready(Ok(map))
        })
        .await
        .context("Failed to query crate names")?;

    // Then, get all verified owner emails for these crates
    let owner_emails = crate_owners::table
        .filter(crate_owners::crate_id.eq_any(crate_ids))
        .filter(crate_owners::owner_kind.eq(OwnerKind::User)) // OwnerKind::User
        .filter(crate_owners::deleted.eq(false))
        .inner_join(emails::table.on(crate_owners::owner_id.eq(emails::user_id)))
        .filter(emails::verified.eq(true))
        .select((crate_owners::crate_id, emails::email))
        .order((emails::email, crate_owners::crate_id))
        .load::<(i32, String)>(conn)
        .await
        .context("Failed to query crate owners")?;

    // Group by email address to send one notification per user
    let mut notifications: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for (crate_id, email) in owner_emails {
        if let Some(crate_name) = crate_id_to_name.get(&crate_id) {
            notifications
                .entry(email)
                .or_default()
                .insert(crate_name.clone());
        }
    }

    // Send notifications in sorted order by email for consistent testing
    for (email, crate_names) in notifications {
        let email_template = TrustedPublishingTokenExposedEmail {
            domain: &state.config.domain_name,
            reporter: "GitHub",
            source: &alert.source,
            crate_names: &crate_names.iter().cloned().collect::<Vec<_>>(),
            url: &alert.url,
        };

        if let Err(error) = state.emails.send(&email, email_template).await {
            warn!(
                %email, ?crate_names, ?error,
                "Failed to send trusted publishing token exposure notification"
            );
        }
    }

    Ok(())
}

struct TrustedPublishingTokenExposedEmail<'a> {
    domain: &'a str,
    reporter: &'a str,
    source: &'a str,
    crate_names: &'a [String],
    url: &'a str,
}

impl Email for TrustedPublishingTokenExposedEmail<'_> {
    fn subject(&self) -> String {
        "crates.io: Your Trusted Publishing token has been revoked".to_string()
    }

    fn body(&self) -> String {
        let authorization = if self.crate_names.len() == 1 {
            format!(
                "This token was only authorized to publish the \"{}\" crate.",
                self.crate_names[0]
            )
        } else {
            format!(
                "This token was authorized to publish the following crates: \"{}\".",
                self.crate_names.join("\", \"")
            )
        };

        let mut body = format!(
            "{reporter} has notified us that one of your crates.io Trusted Publishing tokens \
has been exposed publicly. We have revoked this token as a precaution.

{authorization}

Please review your account at https://{domain} and your GitHub repository \
settings to confirm that no unexpected changes have been made to your crates \
or trusted publishing configurations.

Source type: {source}",
            domain = self.domain,
            reporter = self.reporter,
            source = self.source,
        );

        if self.url.is_empty() {
            body.push_str("\n\nWe were not informed of the URL where the token was found.");
        } else {
            body.push_str(&format!("\n\nURL where the token was found: {}", self.url));
        }

        body.push_str(
            "\n\nTrusted Publishing tokens are temporary and used for automated \
publishing from GitHub Actions. If this exposure was unexpected, please review \
your repository's workflow files and secrets.",
        );

        body
    }
}

#[derive(Deserialize, Serialize)]
pub struct GitHubSecretAlertFeedback {
    pub token_raw: String,
    pub token_type: String,
    pub label: GitHubSecretAlertFeedbackLabel,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GitHubSecretAlertFeedbackLabel {
    TruePositive,
    FalsePositive,
}

/// Handles the `POST /api/github/secret-scanning/verify` route.
pub async fn verify(
    state: AppState,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<Json<Vec<GitHubSecretAlertFeedback>>> {
    verify_github_signature(&headers, &state, &body)
        .await
        .map_err(|e| bad_request(format!("failed to verify request signature: {e:?}")))?;

    let alerts: Vec<GitHubSecretAlert> = json::from_slice(&body)
        .map_err(|e| bad_request(format!("invalid secret alert request: {e:?}")))?;

    let mut conn = state.db_write().await?;

    let mut feedback = Vec::with_capacity(alerts.len());
    for alert in alerts {
        let label = alert_revoke_token(&state, &alert, &mut conn).await?;
        feedback.push(GitHubSecretAlertFeedback {
            token_raw: alert.token,
            token_type: alert.r#type,
            label,
        });
    }

    Ok(Json(feedback))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_cache_valid() {
        assert!(!is_cache_valid(None));
        assert!(!is_cache_valid(Some(
            chrono::Utc::now() - PUBLIC_KEY_CACHE_LIFETIME
        )));
        assert!(is_cache_valid(Some(
            chrono::Utc::now() - (PUBLIC_KEY_CACHE_LIFETIME - Duration::from_secs(1))
        )));
        assert!(is_cache_valid(Some(chrono::Utc::now())));
        // shouldn't happen, but just in case of time travel
        assert!(is_cache_valid(Some(
            chrono::Utc::now() + PUBLIC_KEY_CACHE_LIFETIME
        )));
    }
}
