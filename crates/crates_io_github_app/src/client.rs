use anyhow::Context;
use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use header::HeaderName;
use jsonwebtoken::EncodingKey;
use reqwest::{Client, StatusCode, header};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::debug;
use url::Url;

use crate::jwt::build_jwt;

const DEFAULT_BASE_URL: &str = "https://api.github.com";
const USER_AGENT: &str = "crates.io (https://crates.io)";

/// Header selecting the GitHub REST API version.
const GITHUB_API_VERSION_HEADER: HeaderName = HeaderName::from_static("x-github-api-version");

/// Value of the `X-GitHub-Api-Version` header.
///
/// `2022-11-28` is the first (and default) GitHub REST API version. See
/// <https://docs.github.com/en/rest/overview/api-versions> for the list
/// of currently supported versions.
const GITHUB_API_VERSION: &str = "2022-11-28";

/// Refresh the cached access token when it has less than this much time
/// left before expiry.
const REFRESH_MARGIN: TimeDelta = TimeDelta::minutes(5);

/// Total timeout for any single HTTP request against the GitHub API.
/// 30s guards against a completely hung connection while leaving plenty
/// of headroom for slow TLS or transient congestion.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Timeout for establishing the TCP/TLS connection to GitHub.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Mints installation access tokens for a GitHub App.
#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait GitHubApp: Send + Sync {
    /// Returns a valid installation access token for the configured
    /// organization. Implementations may cache tokens and return the
    /// same value across calls as long as it has not expired.
    async fn installation_token(&self) -> anyhow::Result<SecretString>;
}

/// Production implementation of [`GitHubApp`] that talks to
/// `https://api.github.com`.
pub struct GitHubAppClient {
    client_id: String,
    private_key: EncodingKey,
    org: String,
    api_base_url: Url,
    http: Client,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    installation_id: Option<u64>,
    token: Option<AccessToken>,
}

#[derive(Debug, Deserialize)]
struct AccessToken {
    token: SecretString,
    expires_at: DateTime<Utc>,
}

impl AccessToken {
    /// Instant at which we stop treating the token as fresh. Equal to
    /// the server-reported `expires_at` minus [`REFRESH_MARGIN`], so
    /// callers always have at least that much time to use the token
    /// before it actually expires.
    fn valid_until(&self) -> DateTime<Utc> {
        self.expires_at - REFRESH_MARGIN
    }
}

#[derive(Debug, Deserialize)]
struct InstallationResponse {
    id: u64,
}

impl GitHubAppClient {
    /// Builds a client that talks to `https://api.github.com`.
    ///
    /// `pem` must be the RSA private key issued for the GitHub App, in
    /// PKCS#1 or PKCS#8 PEM form. The key is parsed eagerly and the
    /// constructor fails if it is invalid.
    pub fn new(client_id: &str, pem: &str, org: &str) -> anyhow::Result<Self> {
        let base_url = Url::parse(DEFAULT_BASE_URL)?;
        Self::with_base_url(client_id, pem, org, base_url)
    }

    fn with_base_url(
        client_id: &str,
        pem: &str,
        org: &str,
        api_base_url: Url,
    ) -> anyhow::Result<Self> {
        let private_key = EncodingKey::from_rsa_pem(pem.as_bytes())
            .context("failed to parse GitHub App private key as RSA PEM")?;
        let http = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .context("failed to build reqwest client")?;

        Ok(Self {
            client_id: client_id.to_string(),
            private_key,
            org: org.to_string(),
            api_base_url,
            http,
            state: Mutex::new(State::default()),
        })
    }

    fn jwt(&self) -> anyhow::Result<SecretString> {
        build_jwt(&self.client_id, &self.private_key)
    }

    async fn fetch_installation_id(&self, jwt: &SecretString) -> anyhow::Result<u64> {
        debug!("resolving GitHub App installation id for org {}", self.org);

        let path = format!("orgs/{}/installation", self.org);
        let url = self
            .api_base_url
            .join(&path)
            .context("failed to build installation lookup URL")?;

        let response = self
            .http
            .get(url)
            .bearer_auth(jwt.expose_secret())
            .header(header::ACCEPT, "application/vnd.github+json")
            .header(header::USER_AGENT, USER_AGENT)
            .header(GITHUB_API_VERSION_HEADER, GITHUB_API_VERSION)
            .send()
            .await
            .context("installation lookup request failed")?
            .error_for_status()
            .context("installation lookup returned an error status")?;

        let response: InstallationResponse = response
            .json()
            .await
            .context("failed to decode installation lookup response body")?;

        debug!("resolved installation id {}", response.id);
        Ok(response.id)
    }

    /// Mints a fresh installation access token. Returns `Ok(None)` if
    /// GitHub reports the installation as no longer existing (HTTP 404),
    /// which typically means the app has been uninstalled or reinstalled
    /// under a new installation id.
    async fn mint_access_token(
        &self,
        jwt: &SecretString,
        installation_id: u64,
    ) -> anyhow::Result<Option<AccessToken>> {
        debug!("minting installation access token for installation {installation_id}");
        let url = self
            .api_base_url
            .join(&format!(
                "app/installations/{installation_id}/access_tokens"
            ))
            .context("failed to build access token URL")?;

        let response = self
            .http
            .post(url)
            .bearer_auth(jwt.expose_secret())
            .header(header::ACCEPT, "application/vnd.github+json")
            .header(header::USER_AGENT, USER_AGENT)
            .header(GITHUB_API_VERSION_HEADER, GITHUB_API_VERSION)
            .send()
            .await
            .context("access token request failed")?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let response = response
            .error_for_status()
            .context("access token request returned an error status")?;

        let access_token: AccessToken = response
            .json()
            .await
            .context("failed to decode access token response body")?;

        debug!(
            "minted installation access token for installation {installation_id} (expires at {})",
            access_token.expires_at
        );
        Ok(Some(access_token))
    }
}

#[async_trait]
impl GitHubApp for GitHubAppClient {
    async fn installation_token(&self) -> anyhow::Result<SecretString> {
        let mut state = self.state.lock().await;

        if let Some(cached) = &state.token
            && Utc::now() < cached.valid_until()
        {
            debug!(
                "reusing cached installation access token (expires at {})",
                cached.expires_at
            );
            return Ok(cached.token.clone());
        }

        let jwt = self.jwt().context("failed to sign GitHub App JWT")?;

        let access_token = match state.installation_id {
            Some(id) => match self.mint_access_token(&jwt, id).await? {
                Some(token) => token,
                None => {
                    debug!(
                        "installation {id} not found; refetching installation id (app may have been reinstalled)"
                    );
                    state.installation_id = None;
                    let new_id = self.fetch_installation_id(&jwt).await?;
                    state.installation_id = Some(new_id);
                    self.mint_access_token(&jwt, new_id).await?.ok_or_else(|| {
                        anyhow::anyhow!("installation {new_id} not found immediately after lookup")
                    })?
                }
            },
            None => {
                let id = self.fetch_installation_id(&jwt).await?;
                state.installation_id = Some(id);
                self.mint_access_token(&jwt, id).await?.ok_or_else(|| {
                    anyhow::anyhow!("installation {id} not found immediately after lookup")
                })?
            }
        };

        let value = access_token.token.clone();
        state.token = Some(access_token);
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_keys::TEST_PRIVATE_KEY_PEM;
    use mockito::{Matcher, Server, ServerOpts};
    use secrecy::ExposeSecret;

    const CLIENT_ID: &str = "Iv1.test";
    const ORG: &str = "rust-lang";

    /// Spawns a mockito server whose mocks auto-assert their expected
    /// hit counts when dropped, so tests do not have to sprinkle
    /// `.assert_async()` calls at the end.
    async fn mock_server() -> Server {
        Server::new_with_opts_async(ServerOpts {
            assert_on_drop: true,
            ..Default::default()
        })
        .await
    }

    fn client_with_server(server: &Server) -> GitHubAppClient {
        let api_base_url = Url::parse(&server.url()).unwrap();
        GitHubAppClient::with_base_url(CLIENT_ID, TEST_PRIVATE_KEY_PEM, ORG, api_base_url).unwrap()
    }

    fn access_token_body(token: &str, expires_at: DateTime<Utc>) -> String {
        format!(
            r#"{{"token": "{}", "expires_at": "{}"}}"#,
            token,
            expires_at.to_rfc3339()
        )
    }

    #[tokio::test]
    async fn cached_token_reused_across_calls() {
        let mut server = mock_server().await;
        let expires_at = Utc::now() + TimeDelta::hours(1);

        let _installation_mock = server
            .mock("GET", "/orgs/rust-lang/installation")
            .match_header("authorization", Matcher::Regex("Bearer .+".into()))
            .with_status(200)
            .with_body(r#"{"id": 42}"#)
            .expect(1)
            .create_async()
            .await;

        let _token_mock = server
            .mock("POST", "/app/installations/42/access_tokens")
            .match_header("authorization", Matcher::Regex("Bearer .+".into()))
            .with_status(201)
            .with_body(access_token_body("ghs_first", expires_at))
            .expect(1)
            .create_async()
            .await;

        let client = client_with_server(&server);

        let first = client.installation_token().await.unwrap();
        assert_eq!(first.expose_secret(), "ghs_first");

        let second = client.installation_token().await.unwrap();
        assert_eq!(second.expose_secret(), "ghs_first");
    }

    #[tokio::test]
    async fn installation_lookup_happens_once() {
        let mut server = mock_server().await;

        // Both tokens expire inside the refresh margin, forcing the
        // second call to mint a fresh token while reusing the cached
        // installation id.
        let soon = Utc::now() + TimeDelta::minutes(1);

        let _installation_mock = server
            .mock("GET", "/orgs/rust-lang/installation")
            .with_status(200)
            .with_body(r#"{"id": 42}"#)
            .expect(1)
            .create_async()
            .await;

        let _token_mock = server
            .mock("POST", "/app/installations/42/access_tokens")
            .with_status(201)
            .with_body(access_token_body("ghs_fresh", soon))
            .expect(2)
            .create_async()
            .await;

        let client = client_with_server(&server);

        let first = client.installation_token().await.unwrap();
        assert_eq!(first.expose_secret(), "ghs_fresh");

        let second = client.installation_token().await.unwrap();
        assert_eq!(second.expose_secret(), "ghs_fresh");
    }

    #[tokio::test]
    async fn refetches_installation_id_after_404() {
        let mut server = mock_server().await;
        let soon = Utc::now() + TimeDelta::minutes(1);
        let far = Utc::now() + TimeDelta::hours(1);

        // First call primes the cache with installation id 42 and a
        // short-lived token (expires inside the refresh margin, so the
        // next call re-mints).
        let _get_old = server
            .mock("GET", "/orgs/rust-lang/installation")
            .with_status(200)
            .with_body(r#"{"id": 42}"#)
            .expect(1)
            .create_async()
            .await;

        let _post_42_ok = server
            .mock("POST", "/app/installations/42/access_tokens")
            .with_status(201)
            .with_body(access_token_body("ghs_pre_reinstall", soon))
            .expect(1)
            .create_async()
            .await;

        // Second call: cached installation 42 now returns 404, forcing
        // a re-lookup (GET returns the new id 99), then a successful
        // mint against the new installation.
        let _post_42_gone = server
            .mock("POST", "/app/installations/42/access_tokens")
            .with_status(404)
            .with_body(r#"{"message":"Not Found"}"#)
            .expect(1)
            .create_async()
            .await;

        let _get_new = server
            .mock("GET", "/orgs/rust-lang/installation")
            .with_status(200)
            .with_body(r#"{"id": 99}"#)
            .expect(1)
            .create_async()
            .await;

        let _post_99 = server
            .mock("POST", "/app/installations/99/access_tokens")
            .with_status(201)
            .with_body(access_token_body("ghs_post_reinstall", far))
            .expect(1)
            .create_async()
            .await;

        let client = client_with_server(&server);

        let first = client.installation_token().await.unwrap();
        assert_eq!(first.expose_secret(), "ghs_pre_reinstall");

        let second = client.installation_token().await.unwrap();
        assert_eq!(second.expose_secret(), "ghs_post_reinstall");
    }

    #[tokio::test]
    async fn rejects_invalid_pem() {
        let server = mock_server().await;
        let base_url = Url::parse(&server.url()).unwrap();
        let client = GitHubAppClient::with_base_url(CLIENT_ID, "not a pem", ORG, base_url);
        let err = client.err().expect("expected invalid PEM to be rejected");
        insta::assert_snapshot!(err, @"failed to parse GitHub App private key as RSA PEM");
    }

    /// Smoke-test that the `mockall::automock`-generated `MockGitHubApp`
    /// still compiles and matches the trait. Without this, signature
    /// changes or mockall upgrades only surface downstream.
    #[cfg(feature = "mock")]
    #[tokio::test]
    async fn mock_github_app_compiles() {
        let mut mock = MockGitHubApp::new();
        mock.expect_installation_token()
            .returning(|| Ok(SecretString::from("test-token")));

        let app: &dyn GitHubApp = &mock;
        let token = app.installation_token().await.unwrap();
        assert_eq!(token.expose_secret(), "test-token");
    }
}
