//! [`OAuthProvider`] implementation backed by GitHub OAuth2.
//!
//! This wraps the existing GitHub OAuth2 client ([`BasicClient`]) and the
//! [`GitHubClient`] API client that were already present in [`crate::app::App`].
//! No new HTTP plumbing is introduced here — this is purely a delegation layer
//! that maps GitHub-specific types to the provider-agnostic trait surface.

use std::sync::Arc;

use async_trait::async_trait;
use crates_io_github::{GitHubClient, GitHubError};
use oauth2::basic::{BasicClient, BasicErrorResponseType};
use oauth2::{AccessToken, AuthorizationCode, CsrfToken, EndpointNotSet, EndpointSet, RequestTokenError, Scope, TokenResponse};
use url::Url;

use crate::util::oauth::ReqwestClient;

use super::provider::{OAuthProvider, ProviderError, UserInfo};

/// Type alias matching the field type in [`crate::app::App`].
pub type GithubBasicClient =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

pub struct GitHubProvider {
    oauth: GithubBasicClient,
    client: Arc<dyn GitHubClient>,
    http: reqwest::Client,
}

impl GitHubProvider {
    pub fn new(
        oauth: GithubBasicClient,
        client: Arc<dyn GitHubClient>,
        http: reqwest::Client,
    ) -> Self {
        Self { oauth, client, http }
    }
}

#[async_trait]
impl OAuthProvider for GitHubProvider {
    fn name(&self) -> &'static str {
        "github"
    }

    fn authorize_url(&self) -> (Url, CsrfToken) {
        self.oauth
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("read:org".to_string()))
            .url()
    }

    async fn exchange_code(&self, code: &str) -> Result<AccessToken, ProviderError> {
        let http = ReqwestClient(self.http.clone());
        let token_result = self
            .oauth
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .request_async(&http)
            .await;

        match token_result {
            Ok(response) => Ok(response.access_token().clone()),
            Err(RequestTokenError::Request(e)) => Err(ProviderError::Transient {
                source: Box::new(e),
            }),
            Err(RequestTokenError::ServerResponse(resp)) => {
                // check if this is an "invalid code" error via direct enum matching
                let is_invalid_code = matches!(resp.error(), BasicErrorResponseType::InvalidGrant)
                    || matches!(resp.error(), BasicErrorResponseType::Extension(s) if s == "bad_verification_code");

                if is_invalid_code {
                    Err(ProviderError::InvalidCode)
                } else {
                    // Format the server error as a string — `StandardErrorResponse`
                    // implements `Display` but not `std::error::Error`.
                    Err(ProviderError::Malformed(format!("{resp}")))
                }
            }
            Err(RequestTokenError::Parse(e, _bytes)) => {
                Err(ProviderError::Malformed(e.to_string()))
            }
            // The `RequestTokenError` enum is non-exhaustive; any future
            // variant is treated as a transient infrastructure error.
            Err(e) => Err(ProviderError::Malformed(e.to_string())),
        }
    }

    async fn fetch_user_info(&self, token: &AccessToken) -> Result<UserInfo, ProviderError> {
        match self.client.current_user(token).await {
            Ok(gh) => Ok(UserInfo {
                account_id: gh.id.to_string(),
                login: gh.login,
                name: gh.name,
                avatar_url: gh.avatar_url,
                email: gh.email,
            }),
            Err(GitHubError::Unauthorized(_)) => Err(ProviderError::Unauthorized),
            Err(e) => Err(ProviderError::Transient {
                source: anyhow::Error::from(e).into(),
            }),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crates_io_github::{GitHubError, GitHubUser, MockGitHubClient};

    fn build_test_oauth_client() -> GithubBasicClient {
        use oauth2::{AuthUrl, ClientId, ClientSecret, TokenUrl};
        BasicClient::new(ClientId::new("test-id".to_string()))
            .set_client_secret(ClientSecret::new("test-secret".to_string()))
            .set_auth_uri(
                AuthUrl::new("https://github.com/login/oauth/authorize".into()).unwrap(),
            )
            .set_token_uri(
                TokenUrl::new("https://github.com/login/oauth/access_token".into()).unwrap(),
            )
    }

    fn build_test_provider(mock: MockGitHubClient) -> GitHubProvider {
        GitHubProvider::new(
            build_test_oauth_client(),
            Arc::new(mock),
            reqwest::Client::new(),
        )
    }

    #[test]
    fn name_is_github() {
        let provider = build_test_provider(MockGitHubClient::new());
        assert_eq!(provider.name(), "github");
    }

    #[test]
    fn authorize_url_contains_client_id_and_read_org_scope() {
        let provider = build_test_provider(MockGitHubClient::new());
        let (url, _csrf) = provider.authorize_url();
        let query = url.query().unwrap_or_default();
        assert!(
            query.contains("client_id=test-id"),
            "expected client_id=test-id in query, got: {query}"
        );
        assert!(
            query.contains("scope=read%3Aorg"),
            "expected scope=read%3Aorg in query, got: {query}"
        );
    }

    #[tokio::test]
    async fn fetch_user_info_converts_github_user_to_user_info() {
        let mut mock = MockGitHubClient::new();
        mock.expect_current_user().returning(|_| {
            Ok(GitHubUser {
                id: 42,
                login: "octocat".to_string(),
                name: Some("Octo Cat".to_string()),
                avatar_url: Some("https://example.com/avatar.png".to_string()),
                email: Some("octocat@example.com".to_string()),
            })
        });

        let provider = build_test_provider(mock);
        let token = AccessToken::new("test-token".to_string());
        let info = provider.fetch_user_info(&token).await.unwrap();

        assert_eq!(info.account_id, "42");
        assert_eq!(info.login, "octocat");
        assert_eq!(info.name, Some("Octo Cat".to_string()));
        assert_eq!(
            info.avatar_url,
            Some("https://example.com/avatar.png".to_string())
        );
        assert_eq!(info.email, Some("octocat@example.com".to_string()));
    }

    #[tokio::test]
    async fn fetch_user_info_maps_none_optional_fields() {
        let mut mock = MockGitHubClient::new();
        mock.expect_current_user().returning(|_| {
            Ok(GitHubUser {
                id: 1,
                login: "ghost".to_string(),
                name: None,
                avatar_url: None,
                email: None,
            })
        });

        let provider = build_test_provider(mock);
        let token = AccessToken::new("test-token".to_string());
        let info = provider.fetch_user_info(&token).await.unwrap();

        assert_eq!(info.name, None);
        assert_eq!(info.avatar_url, None);
        assert_eq!(info.email, None);
    }

    #[tokio::test]
    async fn fetch_user_info_maps_401_to_unauthorized() {
        let mut mock = MockGitHubClient::new();
        mock.expect_current_user()
            .returning(|_| Err(GitHubError::Unauthorized(anyhow::anyhow!("401 Unauthorized"))));

        let provider = build_test_provider(mock);
        let token = AccessToken::new("bad-token".to_string());
        let err = provider.fetch_user_info(&token).await.unwrap_err();

        assert!(
            matches!(err, ProviderError::Unauthorized),
            "expected Unauthorized, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn fetch_user_info_maps_other_errors_to_transient() {
        let mut mock = MockGitHubClient::new();
        mock.expect_current_user()
            .returning(|_| Err(GitHubError::Other(anyhow::anyhow!("500 server died"))));

        let provider = build_test_provider(mock);
        let token = AccessToken::new("test-token".to_string());
        let err = provider.fetch_user_info(&token).await.unwrap_err();

        assert!(
            matches!(err, ProviderError::Transient { .. }),
            "expected Transient, got: {err:?}"
        );
    }
}
