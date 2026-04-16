use crate::util::{RequestHelper, TestApp};
use async_trait::async_trait;
use insta::assert_snapshot;
use oauth2::{AccessToken, CsrfToken};
use serde::Deserialize;
use std::sync::Arc;
use url::Url;

use crates_io::oauth::github_provider::PROVIDER_NAME;
use crates_io::oauth::provider::{OAuthProvider, ProviderError, UserInfo};

#[derive(Deserialize)]
struct AuthResponse {
    url: String,
    state: String,
}

/// A minimal concrete [`OAuthProvider`] usable in integration tests where
/// `#[cfg_attr(test, mockall::automock)]` is not in scope.
struct StubGitHubProvider;

#[async_trait]
impl OAuthProvider for StubGitHubProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn authorize_url(&self) -> (Url, CsrfToken) {
        let url = Url::parse(
            "https://github.com/login/oauth/authorize?client_id=test&state=test_csrf_token",
        )
        .unwrap();
        let csrf = CsrfToken::new("test_csrf_token".to_string());
        (url, csrf)
    }

    async fn exchange_code(&self, _code: &str) -> Result<AccessToken, ProviderError> {
        unimplemented!("not needed for begin tests")
    }

    async fn fetch_user_info(&self, _token: &AccessToken) -> Result<UserInfo, ProviderError> {
        unimplemented!("not needed for begin tests")
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn auth_gives_a_token() {
    let (_, anon) = TestApp::init()
        .with_oauth_provider(Arc::new(StubGitHubProvider))
        .empty()
        .await;
    let json: AuthResponse = anon.get("/api/private/session/begin").await.good();
    assert!(
        json.url.contains(&json.state),
        "url '{}' should contain state '{}'",
        json.url,
        json.state
    );
}

/// Without `?provider=` the default is `"github"` — backward compatibility.
#[tokio::test(flavor = "multi_thread")]
async fn begin_defaults_to_github_provider() {
    let (_, anon) = TestApp::init()
        .with_oauth_provider(Arc::new(StubGitHubProvider))
        .empty()
        .await;
    let json: AuthResponse = anon.get("/api/private/session/begin").await.good();
    // The stub returns a GitHub-shaped URL
    assert!(
        json.url.contains("github.com"),
        "expected github.com URL, got: {}",
        json.url
    );
}

/// Explicitly requesting the `github` provider also works.
#[tokio::test(flavor = "multi_thread")]
async fn begin_with_explicit_provider_github() {
    let (_, anon) = TestApp::init()
        .with_oauth_provider(Arc::new(StubGitHubProvider))
        .empty()
        .await;
    let json: AuthResponse = anon
        .get("/api/private/session/begin?provider=github")
        .await
        .good();
    assert!(json.url.contains("github.com"));
}

/// Requesting an unknown provider returns 404.
#[tokio::test(flavor = "multi_thread")]
async fn begin_with_unknown_provider_returns_404() {
    // Empty registry — no providers registered.
    let (_, anon) = TestApp::init().empty().await;
    let response = anon
        .get::<()>("/api/private/session/begin?provider=unknown_provider")
        .await;
    assert_snapshot!(response.status(), @"404 Not Found");
}
