//! Provider-agnostic OAuth abstraction.
//!
//! Implementations wrap a concrete OAuth2 client and whatever HTTP client
//! is needed to fetch the authenticated user's profile. The session
//! controller dispatches through an `Arc<dyn OAuthProvider>` obtained from
//! [`super::registry::ProviderRegistry`].

use async_trait::async_trait;
use oauth2::{AccessToken, CsrfToken};
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("OAuth code exchange was rejected by the upstream provider")]
    InvalidCode,
    #[error("provided access token was rejected by the upstream provider")]
    Unauthorized,
    #[error("upstream response was malformed: {0}")]
    Malformed(String),
    #[error("transient error talking to upstream provider: {source}")]
    Transient {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },
}

/// Provider-agnostic user profile returned by [`OAuthProvider::fetch_user_info`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserInfo {
    /// Stable, unique identifier for this user on the upstream provider.
    ///
    /// Typed as `String` because different providers use incompatible ID
    /// formats: GitHub uses 64-bit integers, but Bitbucket (and other
    /// Atlassian products) use UUIDs for GDPR reasons, and GitLab uses
    /// numeric IDs that are not guaratneed to fit in signed i64. Each
    /// provider's `oauth_<provider>` storage table is free to use whatever
    /// column type is natural (e.g. BIGINT for github, TEXT for bitbucket);
    /// provider implementations convert at the trait boundary.
    pub account_id: String,
    pub login: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub email: Option<String>,
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait OAuthProvider: Send + Sync + 'static {
    /// Stable machine name. Used as:
    /// - the discriminator in `?provider=<name>` query params,
    /// - the suffix in `oauth_<name>` storage table names,
    /// - the key in [`super::registry::ProviderRegistry`].
    fn name(&self) -> &'static str;

    /// Build the authorization URL and CSRF token for the OAuth "begin" step.
    /// Scopes are provider-specific and baked into the impl.
    fn authorize_url(&self) -> (Url, CsrfToken);

    /// Exchange an authorization code for an access token.
    async fn exchange_code(&self, code: &str) -> Result<AccessToken, ProviderError>;

    /// Fetch the authenticated user's profile using a token.
    async fn fetch_user_info(&self, token: &AccessToken) -> Result<UserInfo, ProviderError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_info_with_different_account_ids_are_not_equal() {
        let a = UserInfo {
            account_id: "42".to_string(),
            login: "alice".to_string(),
            name: None,
            avatar_url: None,
            email: None,
        };
        let b = UserInfo {
            account_id: "99".to_string(),
            login: "alice".to_string(), // same login, different account_id
            name: None,
            avatar_url: None,
            email: None,
        };
        // Two users with different account_ids are distinct even if login matches.
        // The session controller depends on account_id for identity, not login.
        assert_ne!(a, b);
        // Clone must preserve all fields including account_id.
        assert_eq!(a, a.clone());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn mock_provider_can_be_boxed() {
        // Confirms the trait is object-safe under #[async_trait] + mockall.
        let mut mock = MockOAuthProvider::new();
        mock.expect_name().return_const("mock");
        let boxed: Box<dyn OAuthProvider> = Box::new(mock);
        assert_eq!(boxed.name(), "mock");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn mock_provider_exchange_code_error_propagates() {
        let mut mock = MockOAuthProvider::new();
        mock.expect_exchange_code()
            .returning(|_| Err(ProviderError::InvalidCode));

        let err = mock.exchange_code("bogus").await.unwrap_err();
        assert!(matches!(err, ProviderError::InvalidCode));
    }
}
