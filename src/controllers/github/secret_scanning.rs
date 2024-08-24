use crate::app::AppState;
use crate::email::Email;
use crate::models::{ApiToken, User};
use crate::schema::api_tokens;
use crate::tasks::spawn_blocking;
use crate::util::diesel::Conn;
use crate::util::errors::{bad_request, AppResult, BoxedAppError};
use crate::util::token::HashedToken;
use anyhow::{anyhow, Context};
use axum::body::Bytes;
use axum::Json;
use base64::{engine::general_purpose, Engine};
use crates_io_github::GitHubPublicKey;
use diesel::prelude::*;
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use http::HeaderMap;
use p256::ecdsa::signature::Verifier;
use p256::ecdsa::VerifyingKey;
use p256::PublicKey;
use serde_json as json;
use std::str::FromStr;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::Mutex;

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
        return Err(bad_request(&format!("unknown key id {req_key_id}")));
    };

    if !key.is_current {
        let error = bad_request(&format!("key id {req_key_id} is not a current key"));
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

/// Revokes an API token and notifies the token owner
fn alert_revoke_token(
    state: &AppState,
    alert: &GitHubSecretAlert,
    conn: &mut impl Conn,
) -> QueryResult<GitHubSecretAlertFeedbackLabel> {
    let hashed_token = HashedToken::hash(&alert.token);

    // Not using `ApiToken::find_by_api_token()` in order to preserve `last_used_at`
    let token = api_tokens::table
        .select(ApiToken::as_select())
        .filter(api_tokens::token.eq(hashed_token))
        .get_result::<ApiToken>(conn)
        .optional()?;

    let Some(token) = token else {
        debug!("Unknown API token received (false positive)");
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
        .execute(conn)?;

    warn!(
        token_id = %token.id, user_id = %token.user_id,
        "Active API token received and revoked (true positive)",
    );

    if let Err(error) = send_notification_email(&token, alert, state, conn) {
        warn!(
            token_id = %token.id, user_id = %token.user_id, ?error,
            "Failed to send email notification",
        )
    }

    Ok(GitHubSecretAlertFeedbackLabel::TruePositive)
}

fn send_notification_email(
    token: &ApiToken,
    alert: &GitHubSecretAlert,
    state: &AppState,
    conn: &mut impl Conn,
) -> anyhow::Result<()> {
    let user = User::find(conn, token.user_id).context("Failed to find user")?;
    let Some(recipient) = user.email(conn)? else {
        return Err(anyhow!("No address found"));
    };

    let email = TokenExposedEmail {
        domain: &state.config.domain_name,
        reporter: "GitHub",
        source: &alert.source,
        token_name: &token.name,
        url: &alert.url,
    };

    state.emails.send(&recipient, email)?;

    Ok(())
}

struct TokenExposedEmail<'a> {
    domain: &'a str,
    reporter: &'a str,
    source: &'a str,
    token_name: &'a str,
    url: &'a str,
}

impl Email for TokenExposedEmail<'_> {
    const SUBJECT: &'static str = "Exposed API token found";

    fn body(&self) -> String {
        let mut body = format!(
            "{reporter} has notified us that your crates.io API token {token_name} \
has been exposed publicly. We have revoked this token as a precaution.

Please review your account at https://{domain} to confirm that no \
unexpected changes have been made to your settings or crates.

Source type: {source}",
            domain = self.domain,
            reporter = self.reporter,
            source = self.source,
            token_name = self.token_name,
        );
        if self.url.is_empty() {
            body.push_str("\n\nWe were not informed of the URL where the token was found.");
        } else {
            body.push_str(&format!("\n\nURL where the token was found: {}", self.url));
        }

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

    let conn = state.db_write().await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        let feedback = alerts
            .into_iter()
            .map(|alert| {
                let label = alert_revoke_token(&state, &alert, conn)?;
                Ok(GitHubSecretAlertFeedback {
                    token_raw: alert.token,
                    token_type: alert.r#type,
                    label,
                })
            })
            .collect::<QueryResult<_>>()?;

        Ok(Json(feedback))
    })
    .await
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
