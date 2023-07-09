use crate::app::AppState;
use crate::controllers::frontend_prelude::*;
use crate::models::{ApiToken, User};
use crate::schema::api_tokens;
use crate::util::token::HashedToken;
use anyhow::{anyhow, Context};
use axum::body::Bytes;
use base64::{engine::general_purpose, Engine};
use http::HeaderMap;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use ring::signature;
use serde_json as json;

static PEM_HEADER: &str = "-----BEGIN PUBLIC KEY-----\n";
static PEM_FOOTER: &str = "\n-----END PUBLIC KEY-----";

// Minimum number of seconds to wait before refreshing cache of GitHub's public keys
static PUBLIC_KEY_CACHE_LIFETIME_SECONDS: i64 = 60 * 60 * 24; // 24 hours

// Cache of public keys that have been fetched from GitHub API
static PUBLIC_KEY_CACHE: Lazy<Mutex<GitHubPublicKeyCache>> = Lazy::new(|| {
    let keys: Vec<GitHubPublicKey> = Vec::new();
    let cache = GitHubPublicKeyCache {
        keys,
        timestamp: None,
    };
    Mutex::new(cache)
});

#[derive(Debug, Deserialize, Clone, Eq, Hash, PartialEq)]
pub struct GitHubPublicKey {
    pub key_identifier: String,
    pub key: String,
    pub is_current: bool,
}

#[derive(Debug, Deserialize)]
pub struct GitHubPublicKeyList {
    pub public_keys: Vec<GitHubPublicKey>,
}

#[derive(Debug, Clone)]
struct GitHubPublicKeyCache {
    keys: Vec<GitHubPublicKey>,
    timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

/// Converts a PEM format ECDSA P-256 SHA-256 public key in SubjectPublicKeyInfo format into
/// the Octet-String-to-Elliptic-Curve-Point format expected by ring::signature::verify
fn key_from_spki(key: &GitHubPublicKey) -> Result<Vec<u8>, std::io::Error> {
    let start_idx = key
        .key
        .find(PEM_HEADER)
        .ok_or(std::io::ErrorKind::InvalidData)?;
    let gh_key = &key.key[(start_idx + PEM_HEADER.len())..];
    let end_idx = gh_key
        .find(PEM_FOOTER)
        .ok_or(std::io::ErrorKind::InvalidData)?;
    let gh_key = gh_key[..end_idx].replace('\n', "");
    let gh_key = general_purpose::STANDARD
        .decode(gh_key)
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?;
    if gh_key.len() != 91 {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
    }
    // extract the key bytes from the fixed position in the ASN.1 structure
    Ok(gh_key[26..91].to_vec())
}

/// Check if cache of public keys is populated and not expired
fn is_cache_valid(timestamp: Option<chrono::DateTime<chrono::Utc>>) -> bool {
    timestamp.is_some()
        && chrono::Utc::now()
            < timestamp.unwrap() + chrono::Duration::seconds(PUBLIC_KEY_CACHE_LIFETIME_SECONDS)
}

// Fetches list of public keys from GitHub API
fn get_public_keys(state: &AppState) -> Result<Vec<GitHubPublicKey>, BoxedAppError> {
    // Return list from cache if populated and still valid
    let mut cache = PUBLIC_KEY_CACHE.lock();
    if is_cache_valid(cache.timestamp) {
        return Ok(cache.keys.clone());
    }

    // Fetch from GitHub API
    let keys = state.github.public_keys(
        &state.config.gh_client_id,
        state.config.gh_client_secret.secret(),
    )?;

    // Populate cache
    cache.keys = keys.clone();
    cache.timestamp = Some(chrono::Utc::now());

    Ok(keys)
}

/// Verifies that the GitHub signature in request headers is valid
fn verify_github_signature(
    headers: &HeaderMap,
    state: &AppState,
    json: &[u8],
) -> Result<(), BoxedAppError> {
    // Read and decode request headers
    let req_key_id = headers
        .get("GITHUB-PUBLIC-KEY-IDENTIFIER")
        .ok_or_else(|| bad_request("missing HTTP header: GITHUB-PUBLIC-KEY-IDENTIFIER"))?
        .to_str()
        .map_err(|e| bad_request(&format!("failed to decode HTTP header: {e:?}")))?;
    let sig = headers
        .get("GITHUB-PUBLIC-KEY-SIGNATURE")
        .ok_or_else(|| bad_request("missing HTTP header: GITHUB-PUBLIC-KEY-SIGNATURE"))?;
    let sig = general_purpose::STANDARD
        .decode(sig)
        .map_err(|e| bad_request(&format!("failed to decode signature as base64: {e:?}")))?;
    let public_keys = get_public_keys(state)
        .map_err(|e| bad_request(&format!("failed to fetch GitHub public keys: {e:?}")))?;

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

    let key_bytes = key_from_spki(key).map_err(|_| bad_request("cannot parse public key"))?;
    let gh_key = signature::UnparsedPublicKey::new(&signature::ECDSA_P256_SHA256_ASN1, &key_bytes);

    gh_key
        .verify(json, &sig)
        .map_err(|e| bad_request(&format!("invalid signature: {e:?}")))?;

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
) -> Result<GitHubSecretAlertFeedbackLabel, BoxedAppError> {
    let conn = &mut *state.db_write()?;

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
    conn: &mut PgConnection,
) -> anyhow::Result<()> {
    let user = User::find(conn, token.user_id).context("Failed to find user")?;
    let Some(email) = user.email(conn)? else {
        return Err(anyhow!("No address found"));
    };

    state
        .emails
        .send_token_exposed_notification(&email, &alert.url, "GitHub", &alert.source, &token.name)
        .map_err(|error| anyhow!("{error}"))?;

    Ok(())
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
    conduit_compat(move || {
        verify_github_signature(&headers, &state, &body)
            .map_err(|e| bad_request(&format!("failed to verify request signature: {e:?}")))?;

        let alerts: Vec<GitHubSecretAlert> = json::from_slice(&body)
            .map_err(|e| bad_request(&format!("invalid secret alert request: {e:?}")))?;

        let feedback = alerts
            .into_iter()
            .map(|alert| {
                let label = alert_revoke_token(&state, &alert)?;
                Ok(GitHubSecretAlertFeedback {
                    token_raw: alert.token,
                    token_type: alert.r#type,
                    label,
                })
            })
            .collect::<Result<_, BoxedAppError>>()?;

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
            chrono::Utc::now() - chrono::Duration::seconds(PUBLIC_KEY_CACHE_LIFETIME_SECONDS)
        )));
        assert!(is_cache_valid(Some(
            chrono::Utc::now() - chrono::Duration::seconds(PUBLIC_KEY_CACHE_LIFETIME_SECONDS - 1)
        )));
        assert!(is_cache_valid(Some(chrono::Utc::now())));
        // shouldn't happen, but just in case of time travel
        assert!(is_cache_valid(Some(
            chrono::Utc::now() + chrono::Duration::seconds(PUBLIC_KEY_CACHE_LIFETIME_SECONDS)
        )));
    }
}
