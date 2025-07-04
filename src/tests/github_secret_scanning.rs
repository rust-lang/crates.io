use crate::tests::builders::CrateBuilder;
use crate::tests::util::MockRequestExt;
use crate::tests::util::insta::api_token_redaction;
use crate::tests::{RequestHelper, TestApp};
use crate::util::token::HashedToken;
use crate::{models::ApiToken, schema::api_tokens};
use base64::{Engine as _, engine::general_purpose};
use chrono::{TimeDelta, Utc};
use claims::assert_ok;
use crates_io_database::models::CrateOwner;
use crates_io_database::models::trustpub::NewToken;
use crates_io_database::schema::trustpub_tokens;
use crates_io_github::{GitHubPublicKey, MockGitHubClient};
use crates_io_trustpub::access_token::AccessToken;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use googletest::prelude::*;
use insta::{assert_json_snapshot, assert_snapshot};
use p256::ecdsa::{Signature, SigningKey, signature::Signer};
use p256::pkcs8::DecodePrivateKey;
use secrecy::ExposeSecret;
use std::sync::LazyLock;

static URL: &str = "/api/github/secret-scanning/verify";

// Test request payload for GitHub secret scanning
static GITHUB_ALERT: &[u8] =
    br#"[{"token":"some_token","type":"some_type","url":"some_url","source":"some_source"}]"#;

/// Generate a GitHub alert with a given token
fn github_alert_with_token(token: &str) -> Vec<u8> {
    format!(
        r#"[{{"token":"{token}","type":"some_type","url":"some_url","source":"some_source"}}]"#,
    )
    .into_bytes()
}

/// Private key for signing payloads (ECDSA P-256)
///
/// Generated specifically for testing - do not use in production.
///
/// This corresponds to the public key below and is used to generate valid signatures
static PRIVATE_KEY: &str = r#"-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgV64BdEFXg9aT/m4p
wOQ/o9WUHxZ6qfBaP3D7Km1TOWuhRANCAARYKkbkTbIr//8klg1CMYGQIwtlfNd4
JQYV5+q0s3+JnBSLb1/sx/lEDzmMVZQIZQrACUHFW4UVdmox2NvmNWyy
-----END PRIVATE KEY-----"#;

/// Public key (corresponds to the private key above)
static PUBLIC_KEY: &str = r#"-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEWCpG5E2yK///JJYNQjGBkCMLZXzX
eCUGFefqtLN/iZwUi29f7Mf5RA85jFWUCGUKwAlBxVuFFXZqMdjb5jVssg==
-----END PUBLIC KEY-----"#;

/// Public key identifier (SHA256 hash of the DER-encoded public key)
static KEY_IDENTIFIER: &str = "2aafbbe2d329af78d875cd2dd0291048799176466844315b6a846d6e12aa26ca";

/// Signing key derived from the private key
static SIGNING_KEY: LazyLock<SigningKey> =
    LazyLock::new(|| SigningKey::from_pkcs8_pem(PRIVATE_KEY).unwrap());

/// Generate a signature for the payload using our private key
fn sign_payload(payload: &[u8]) -> String {
    let signature: Signature = SIGNING_KEY.sign(payload);
    general_purpose::STANDARD.encode(signature.to_der())
}

/// Generate a new Trusted Publishing token and its SHA256 hash
fn generate_trustpub_token() -> (String, Vec<u8>) {
    let token = AccessToken::generate();
    let finalized_token = token.finalize().expose_secret().to_string();
    let hashed_token = token.sha256().to_vec();
    (finalized_token, hashed_token)
}

/// Create a new Trusted Publishing token in the database
async fn insert_trustpub_token(
    conn: &mut diesel_async::AsyncPgConnection,
    crate_ids: &[i32],
) -> QueryResult<String> {
    let (token, hashed_token) = generate_trustpub_token();

    let new_token = NewToken {
        expires_at: Utc::now() + TimeDelta::minutes(30),
        hashed_token: &hashed_token,
        crate_ids,
        trustpub_data: None,
    };

    new_token.insert(conn).await?;

    Ok(token)
}

fn github_mock() -> MockGitHubClient {
    let mut mock = MockGitHubClient::new();

    mock.expect_public_keys().returning(|_, _| {
        let key = GitHubPublicKey {
            key_identifier: KEY_IDENTIFIER.to_string(),
            key: PUBLIC_KEY.to_string(),
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
    request.header("GITHUB-PUBLIC-KEY-IDENTIFIER", KEY_IDENTIFIER);
    request.header("GITHUB-PUBLIC-KEY-SIGNATURE", &sign_payload(GITHUB_ALERT));
    let response = anon.run::<()>(request).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json());

    // Ensure that the token was revoked
    let tokens: Vec<ApiToken> = assert_ok!(
        ApiToken::belonging_to(user.as_model())
            .select(ApiToken::as_select())
            .filter(api_tokens::revoked.eq(false))
            .load(&mut conn)
            .await
    );
    assert_that!(tokens, is_empty());

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
    request.header("GITHUB-PUBLIC-KEY-IDENTIFIER", KEY_IDENTIFIER);
    request.header("GITHUB-PUBLIC-KEY-SIGNATURE", &sign_payload(GITHUB_ALERT));
    let response = anon.run::<()>(request).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json());

    // Ensure that the token is still revoked
    let tokens: Vec<ApiToken> = assert_ok!(
        ApiToken::belonging_to(user.as_model())
            .select(ApiToken::as_select())
            .filter(api_tokens::revoked.eq(false))
            .load(&mut conn)
            .await
    );
    assert_that!(tokens, is_empty());

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
    request.header("GITHUB-PUBLIC-KEY-IDENTIFIER", KEY_IDENTIFIER);
    request.header("GITHUB-PUBLIC-KEY-SIGNATURE", &sign_payload(GITHUB_ALERT));
    let response = anon.run::<()>(request).await;
    assert_snapshot!(response.status(), @"200 OK");
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
    assert_snapshot!(response.status(), @"400 Bad Request");

    // Request body but no headers
    let mut request = anon.post_request(URL);
    *request.body_mut() = GITHUB_ALERT.into();
    let response = anon.run::<()>(request).await;
    assert_snapshot!(response.status(), @"400 Bad Request");

    // Headers but no request body
    let mut request = anon.post_request(URL);
    request.header("GITHUB-PUBLIC-KEY-IDENTIFIER", KEY_IDENTIFIER);
    request.header("GITHUB-PUBLIC-KEY-SIGNATURE", &sign_payload(GITHUB_ALERT));
    let response = anon.run::<()>(request).await;
    assert_snapshot!(response.status(), @"400 Bad Request");

    // Request body but only key identifier header
    let mut request = anon.post_request(URL);
    *request.body_mut() = GITHUB_ALERT.into();
    request.header("GITHUB-PUBLIC-KEY-IDENTIFIER", KEY_IDENTIFIER);
    let response = anon.run::<()>(request).await;
    assert_snapshot!(response.status(), @"400 Bad Request");

    // Invalid signature
    let mut request = anon.post_request(URL);
    *request.body_mut() = GITHUB_ALERT.into();
    request.header("GITHUB-PUBLIC-KEY-IDENTIFIER", KEY_IDENTIFIER);
    request.header("GITHUB-PUBLIC-KEY-SIGNATURE", "bad signature");
    let response = anon.run::<()>(request).await;
    assert_snapshot!(response.status(), @"400 Bad Request");

    // Invalid signature that is valid base64
    let mut request = anon.post_request(URL);
    *request.body_mut() = GITHUB_ALERT.into();
    request.header("GITHUB-PUBLIC-KEY-IDENTIFIER", KEY_IDENTIFIER);
    request.header("GITHUB-PUBLIC-KEY-SIGNATURE", "YmFkIHNpZ25hdHVyZQ==");
    let response = anon.run::<()>(request).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
}

#[tokio::test(flavor = "multi_thread")]
async fn github_secret_alert_revokes_trustpub_token() {
    let (app, anon, cookie) = TestApp::init().with_github(github_mock()).with_user().await;
    let mut conn = app.db_conn().await;

    let krate = CrateBuilder::new("foo", cookie.as_model().id)
        .build(&mut conn)
        .await
        .unwrap();

    // Generate a valid Trusted Publishing token
    let token = insert_trustpub_token(&mut conn, &[krate.id]).await.unwrap();

    // Verify the token exists in the database
    let count = trustpub_tokens::table
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();
    assert_eq!(count, 1);

    // Send the GitHub alert to the API endpoint
    let mut request = anon.post_request(URL);
    let vec = github_alert_with_token(&token);
    request.header("GITHUB-PUBLIC-KEY-IDENTIFIER", KEY_IDENTIFIER);
    request.header("GITHUB-PUBLIC-KEY-SIGNATURE", &sign_payload(&vec));
    *request.body_mut() = vec.into();
    let response = anon.run::<()>(request).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), {
        "[].token_raw" => api_token_redaction()
    });

    // Verify the token was deleted from the database
    let count = trustpub_tokens::table
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();
    assert_eq!(count, 0);

    // Ensure an email was sent notifying about the token revocation
    assert_snapshot!(app.emails_snapshot().await);
}

#[tokio::test(flavor = "multi_thread")]
async fn github_secret_alert_for_unknown_trustpub_token() {
    let (app, anon) = TestApp::init().with_github(github_mock()).empty().await;
    let mut conn = app.db_conn().await;

    // Generate a valid Trusted Publishing token but don't insert it into the database
    let (token, _) = generate_trustpub_token();

    // Verify no tokens exist in the database
    let count = trustpub_tokens::table
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();
    assert_eq!(count, 0);

    // Send the GitHub alert to the API endpoint
    let mut request = anon.post_request(URL);
    let vec = github_alert_with_token(&token);
    request.header("GITHUB-PUBLIC-KEY-IDENTIFIER", KEY_IDENTIFIER);
    request.header("GITHUB-PUBLIC-KEY-SIGNATURE", &sign_payload(&vec));
    *request.body_mut() = vec.into();
    let response = anon.run::<()>(request).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), {
        "[].token_raw" => api_token_redaction()
    });

    // Verify still no tokens exist in the database
    let count = trustpub_tokens::table
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();
    assert_eq!(count, 0);

    // Ensure no emails were sent
    assert_eq!(app.emails().await.len(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn github_secret_alert_revokes_trustpub_token_multiple_users() {
    let (app, anon) = TestApp::init().with_github(github_mock()).empty().await;
    let mut conn = app.db_conn().await;

    // Create two users
    let user1 = app.db_new_user("user1").await;
    let user2 = app.db_new_user("user2").await;

    // Create two crates
    // User 1 owns both crates 1 and 2
    let crate1 = CrateBuilder::new("crate1", user1.as_model().id)
        .build(&mut conn)
        .await
        .unwrap();
    let crate2 = CrateBuilder::new("crate2", user1.as_model().id)
        .build(&mut conn)
        .await
        .unwrap();

    // Add user 2 as owner of crate2
    CrateOwner::builder()
        .crate_id(crate2.id)
        .user_id(user2.as_model().id)
        .created_by(user1.as_model().id)
        .build()
        .insert(&mut conn)
        .await
        .unwrap();

    // Generate a trusted publishing token that has access to both crates
    let token = insert_trustpub_token(&mut conn, &[crate1.id, crate2.id])
        .await
        .unwrap();

    // Send the GitHub alert to the API endpoint
    let mut request = anon.post_request(URL);
    let vec = github_alert_with_token(&token);
    request.header("GITHUB-PUBLIC-KEY-IDENTIFIER", KEY_IDENTIFIER);
    request.header("GITHUB-PUBLIC-KEY-SIGNATURE", &sign_payload(&vec));
    *request.body_mut() = vec.into();
    let response = anon.run::<()>(request).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), {
        "[].token_raw" => api_token_redaction()
    });

    // Take a snapshot of all emails for detailed verification
    assert_snapshot!(app.emails_snapshot().await);
}
