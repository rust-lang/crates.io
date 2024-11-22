use crate::tests::util::MockRequestExt;
use crate::tests::{RequestHelper, TestApp};
use crate::util::token::HashedToken;
use crate::{models::ApiToken, schema::api_tokens};
use crates_io_github::{GitHubPublicKey, MockGitHubClient};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use googletest::prelude::*;
use http::StatusCode;
use insta::{assert_json_snapshot, assert_snapshot};

static URL: &str = "/api/github/secret-scanning/verify";

// Test request and signature from https://docs.github.com/en/developers/overview/secret-scanning-partner-program#create-a-secret-alert-service
static GITHUB_ALERT: &[u8] =
    br#"[{"token":"some_token","type":"some_type","url":"some_url","source":"some_source"}]"#;

static GITHUB_PUBLIC_KEY_IDENTIFIER: &str =
    "f9525bf080f75b3506ca1ead061add62b8633a346606dc5fe544e29231c6ee0d";

/// Test key from https://docs.github.com/en/developers/overview/secret-scanning-partner-program#create-a-secret-alert-service
static GITHUB_PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----\nMFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEsz9ugWDj5jK5ELBK42ynytbo38gP\nHzZFI03Exwz8Lh/tCfL3YxwMdLjB+bMznsanlhK0RwcGP3IDb34kQDIo3Q==\n-----END PUBLIC KEY-----";

static GITHUB_PUBLIC_KEY_SIGNATURE: &str = "MEUCIFLZzeK++IhS+y276SRk2Pe5LfDrfvTXu6iwKKcFGCrvAiEAhHN2kDOhy2I6eGkOFmxNkOJ+L2y8oQ9A2T9GGJo6WJY=";

fn github_mock() -> MockGitHubClient {
    let mut mock = MockGitHubClient::new();

    mock.expect_public_keys().returning(|_, _| {
        let key = GitHubPublicKey {
            key_identifier: GITHUB_PUBLIC_KEY_IDENTIFIER.to_string(),
            key: GITHUB_PUBLIC_KEY.to_string(),
            is_current: true,
        };

        Ok(vec![key])
    });

    mock
}

#[tokio::test(flavor = "multi_thread")]
async fn github_secret_alert_revokes_token() {
    let (app, anon, user, token) = TestApp::init()
        .with_github(github_mock())
        .with_token()
        .await;
    let mut conn = app.db_conn().await;

    // Ensure no emails were sent up to this point
    assert_eq!(app.emails().await.len(), 0);

    // Ensure that the token currently exists in the database
    let tokens: Vec<ApiToken> = assert_ok!(
        ApiToken::belonging_to(user.as_model())
            .select(ApiToken::as_select())
            .filter(api_tokens::revoked.eq(false))
            .load(&mut conn)
            .await
    );
    assert_that!(tokens, len(eq(1)));
    assert_eq!(tokens[0].name, token.as_model().name);

    // Set token to expected value in signed request
    let hashed_token = HashedToken::hash("some_token");
    diesel::update(api_tokens::table)
        .set(api_tokens::token.eq(hashed_token))
        .execute(&mut conn)
        .await
        .unwrap();

    let mut request = anon.post_request(URL);
    *request.body_mut() = GITHUB_ALERT.into();
    request.header("GITHUB-PUBLIC-KEY-IDENTIFIER", GITHUB_PUBLIC_KEY_IDENTIFIER);
    request.header("GITHUB-PUBLIC-KEY-SIGNATURE", GITHUB_PUBLIC_KEY_SIGNATURE);
    let response = anon.run::<()>(request).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());

    // Ensure that the token was revoked
    let tokens: Vec<ApiToken> = assert_ok!(
        ApiToken::belonging_to(user.as_model())
            .select(ApiToken::as_select())
            .filter(api_tokens::revoked.eq(false))
            .load(&mut conn)
            .await
    );
    assert_that!(tokens, empty());

    let tokens: Vec<ApiToken> = assert_ok!(
        ApiToken::belonging_to(user.as_model())
            .select(ApiToken::as_select())
            .filter(api_tokens::revoked.eq(true))
            .load(&mut conn)
            .await
    );
    assert_that!(tokens, len(eq(1)));

    // Ensure exactly one email was sent
    assert_snapshot!(app.emails_snapshot().await);
}

#[tokio::test(flavor = "multi_thread")]
async fn github_secret_alert_for_revoked_token() {
    let (app, anon, user, token) = TestApp::init()
        .with_github(github_mock())
        .with_token()
        .await;
    let mut conn = app.db_conn().await;

    // Ensure no emails were sent up to this point
    assert_eq!(app.emails().await.len(), 0);

    // Ensure that the token currently exists in the database
    let tokens: Vec<ApiToken> = assert_ok!(
        ApiToken::belonging_to(user.as_model())
            .select(ApiToken::as_select())
            .filter(api_tokens::revoked.eq(false))
            .load(&mut conn)
            .await
    );
    assert_that!(tokens, len(eq(1)));
    assert_eq!(tokens[0].name, token.as_model().name);

    // Set token to expected value in signed request and revoke it
    let hashed_token = HashedToken::hash("some_token");
    diesel::update(api_tokens::table)
        .set((
            api_tokens::token.eq(hashed_token),
            api_tokens::revoked.eq(true),
        ))
        .execute(&mut conn)
        .await
        .unwrap();

    let mut request = anon.post_request(URL);
    *request.body_mut() = GITHUB_ALERT.into();
    request.header("GITHUB-PUBLIC-KEY-IDENTIFIER", GITHUB_PUBLIC_KEY_IDENTIFIER);
    request.header("GITHUB-PUBLIC-KEY-SIGNATURE", GITHUB_PUBLIC_KEY_SIGNATURE);
    let response = anon.run::<()>(request).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());

    // Ensure that the token is still revoked
    let tokens: Vec<ApiToken> = assert_ok!(
        ApiToken::belonging_to(user.as_model())
            .select(ApiToken::as_select())
            .filter(api_tokens::revoked.eq(false))
            .load(&mut conn)
            .await
    );
    assert_that!(tokens, empty());

    let tokens: Vec<ApiToken> = assert_ok!(
        ApiToken::belonging_to(user.as_model())
            .select(ApiToken::as_select())
            .filter(api_tokens::revoked.eq(true))
            .load(&mut conn)
            .await
    );
    assert_that!(tokens, len(eq(1)));

    // Ensure still no emails were sent
    assert_eq!(app.emails().await.len(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn github_secret_alert_for_unknown_token() {
    let (app, anon, user, token) = TestApp::init()
        .with_github(github_mock())
        .with_token()
        .await;
    let mut conn = app.db_conn().await;

    // Ensure no emails were sent up to this point
    assert_eq!(app.emails().await.len(), 0);

    // Ensure that the token currently exists in the database
    let tokens: Vec<ApiToken> = assert_ok!(
        ApiToken::belonging_to(user.as_model())
            .select(ApiToken::as_select())
            .filter(api_tokens::revoked.eq(false))
            .load(&mut conn)
            .await
    );
    assert_that!(tokens, len(eq(1)));
    assert_eq!(tokens[0].name, token.as_model().name);

    let mut request = anon.post_request(URL);
    *request.body_mut() = GITHUB_ALERT.into();
    request.header("GITHUB-PUBLIC-KEY-IDENTIFIER", GITHUB_PUBLIC_KEY_IDENTIFIER);
    request.header("GITHUB-PUBLIC-KEY-SIGNATURE", GITHUB_PUBLIC_KEY_SIGNATURE);
    let response = anon.run::<()>(request).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());

    // Ensure that the token was not revoked
    let tokens: Vec<ApiToken> = assert_ok!(
        ApiToken::belonging_to(user.as_model())
            .select(ApiToken::as_select())
            .filter(api_tokens::revoked.eq(false))
            .load(&mut conn)
            .await
    );
    assert_that!(tokens, len(eq(1)));
    assert_eq!(tokens[0].name, token.as_model().name);

    // Ensure still no emails were sent
    assert_eq!(app.emails().await.len(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn github_secret_alert_invalid_signature_fails() {
    let (_, anon) = TestApp::init().with_github(github_mock()).empty().await;

    // No headers or request body
    let request = anon.post_request(URL);
    let response = anon.run::<()>(request).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Request body but no headers
    let mut request = anon.post_request(URL);
    *request.body_mut() = GITHUB_ALERT.into();
    let response = anon.run::<()>(request).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Headers but no request body
    let mut request = anon.post_request(URL);
    request.header("GITHUB-PUBLIC-KEY-IDENTIFIER", GITHUB_PUBLIC_KEY_IDENTIFIER);
    request.header("GITHUB-PUBLIC-KEY-SIGNATURE", GITHUB_PUBLIC_KEY_SIGNATURE);
    let response = anon.run::<()>(request).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Request body but only key identifier header
    let mut request = anon.post_request(URL);
    *request.body_mut() = GITHUB_ALERT.into();
    request.header("GITHUB-PUBLIC-KEY-IDENTIFIER", GITHUB_PUBLIC_KEY_IDENTIFIER);
    let response = anon.run::<()>(request).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Invalid signature
    let mut request = anon.post_request(URL);
    *request.body_mut() = GITHUB_ALERT.into();
    request.header("GITHUB-PUBLIC-KEY-IDENTIFIER", GITHUB_PUBLIC_KEY_IDENTIFIER);
    request.header("GITHUB-PUBLIC-KEY-SIGNATURE", "bad signature");
    let response = anon.run::<()>(request).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Invalid signature that is valid base64
    let mut request = anon.post_request(URL);
    *request.body_mut() = GITHUB_ALERT.into();
    request.header("GITHUB-PUBLIC-KEY-IDENTIFIER", GITHUB_PUBLIC_KEY_IDENTIFIER);
    request.header("GITHUB-PUBLIC-KEY-SIGNATURE", "YmFkIHNpZ25hdHVyZQ==");
    let response = anon.run::<()>(request).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
