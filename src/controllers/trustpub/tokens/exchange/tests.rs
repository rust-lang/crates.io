use crate::tests::builders::CrateBuilder;
use crate::tests::util::{MockAnonymousUser, RequestHelper, TestApp};
use crates_io_database::models::trustpub::NewGitHubConfig;
use crates_io_database::schema::trustpub_tokens;
use crates_io_trustpub::access_token::AccessToken;
use crates_io_trustpub::github::GITHUB_ISSUER_URL;
use crates_io_trustpub::github::test_helpers::FullGitHubClaims;
use crates_io_trustpub::keystore::MockOidcKeyStore;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use insta::{assert_compact_debug_snapshot, assert_json_snapshot, assert_snapshot};
use jsonwebtoken::{EncodingKey, Header};
use mockall::predicate::*;
use serde_json::json;

const URL: &str = "/api/v1/trusted_publishing/tokens";

const CRATE_NAME: &str = "foo";
const OWNER_NAME: &str = "rust-lang";
const OWNER_ID: i32 = 42;
const REPOSITORY_NAME: &str = "foo-rs";
const WORKFLOW_FILENAME: &str = "publish.yml";

async fn prepare() -> anyhow::Result<MockAnonymousUser> {
    prepare_with_config(|_config| {}).await
}

async fn prepare_with_config(
    adjust_config: fn(&mut NewGitHubConfig<'static>),
) -> anyhow::Result<MockAnonymousUser> {
    let (app, client, cookie) = TestApp::full()
        .with_oidc_keystore(GITHUB_ISSUER_URL, MockOidcKeyStore::with_test_key())
        .with_user()
        .await;

    let mut conn = app.db_conn().await;

    let owner_id = cookie.as_model().id;
    let krate = CrateBuilder::new(CRATE_NAME, owner_id)
        .build(&mut conn)
        .await?;

    let mut new_oidc_config = new_oidc_config(krate.id);
    adjust_config(&mut new_oidc_config);
    new_oidc_config.insert(&mut conn).await?;

    Ok(client)
}

fn new_oidc_config(crate_id: i32) -> NewGitHubConfig<'static> {
    NewGitHubConfig {
        crate_id,
        repository_owner: OWNER_NAME,
        repository_owner_id: OWNER_ID,
        repository_name: REPOSITORY_NAME,
        workflow_filename: WORKFLOW_FILENAME,
        environment: None,
    }
}

fn default_claims() -> FullGitHubClaims {
    FullGitHubClaims::builder()
        .owner_id(OWNER_ID)
        .owner_name(OWNER_NAME)
        .repository_name(REPOSITORY_NAME)
        .workflow_filename(WORKFLOW_FILENAME)
        .build()
}

#[tokio::test(flavor = "multi_thread")]
async fn test_happy_path() -> anyhow::Result<()> {
    let client = prepare().await?;

    let body = default_claims().as_exchange_body()?;
    let response = client.put::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"200 OK");

    let json = response.json();
    assert_json_snapshot!(json, { ".token" => "[token]" }, @r#"
    {
      "token": "[token]"
    }
    "#);

    let token = json["token"].as_str().unwrap();
    let token = assert_ok!(AccessToken::from_byte_str(token.as_bytes()));
    let hashed_token = token.sha256();

    let mut conn = client.app().db_conn().await;

    let tokens = trustpub_tokens::table
        .filter(trustpub_tokens::hashed_token.eq(hashed_token.as_slice()))
        .select((trustpub_tokens::id, trustpub_tokens::crate_ids))
        .get_results::<(i64, Vec<Option<i32>>)>(&mut conn)
        .await?;

    assert_eq!(tokens.len(), 1);
    assert_compact_debug_snapshot!(tokens, @"[(1, [Some(1)])]");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_happy_path_with_environment() -> anyhow::Result<()> {
    let client = prepare_with_config(|c| c.environment = Some("prod")).await?;

    let mut claims = default_claims();
    claims.environment = Some("prod".into());

    let body = claims.as_exchange_body()?;
    let response = client.put::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"200 OK");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_happy_path_with_ignored_environment() -> anyhow::Result<()> {
    let client = prepare().await?;

    let mut claims = default_claims();
    claims.environment = Some("prod".into());

    let body = claims.as_exchange_body()?;
    let response = client.put::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"200 OK");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_broken_jwt() -> anyhow::Result<()> {
    let client = prepare().await?;

    let body = serde_json::to_vec(&json!({ "jwt": "broken" }))?;
    let response = client.put::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"Failed to decode JWT"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unsupported_issuer() -> anyhow::Result<()> {
    let (app, client, cookie) = TestApp::full().with_user().await;

    let mut conn = app.db_conn().await;

    let owner_id = cookie.as_model().id;
    let krate = CrateBuilder::new(CRATE_NAME, owner_id)
        .build(&mut conn)
        .await?;

    new_oidc_config(krate.id).insert(&mut conn).await?;

    let body = default_claims().as_exchange_body()?;
    let response = client.put::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"Unsupported JWT issuer"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_missing_key_id() -> anyhow::Result<()> {
    let client = prepare().await?;

    let claims = default_claims();
    let secret_key = EncodingKey::from_secret(b"secret");
    let jwt = jsonwebtoken::encode(&Header::default(), &claims, &secret_key)?;
    let body = serde_json::to_vec(&json!({ "jwt": jwt }))?;

    let response = client.put::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"Missing JWT key ID"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unknown_key() -> anyhow::Result<()> {
    let mut mock_key_store = MockOidcKeyStore::default();

    mock_key_store
        .expect_get_oidc_key()
        .with(always())
        .returning(|_| Ok(None));

    let (app, client, cookie) = TestApp::full()
        .with_oidc_keystore(GITHUB_ISSUER_URL, mock_key_store)
        .with_user()
        .await;

    let mut conn = app.db_conn().await;

    let owner_id = cookie.as_model().id;
    let krate = CrateBuilder::new("foo", owner_id).build(&mut conn).await?;
    new_oidc_config(krate.id).insert(&mut conn).await?;

    let body = default_claims().as_exchange_body()?;
    let response = client.put::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"Invalid JWT key ID"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_key_store_error() -> anyhow::Result<()> {
    let mut mock_key_store = MockOidcKeyStore::default();

    mock_key_store
        .expect_get_oidc_key()
        .with(always())
        .returning(|_| Err(anyhow::anyhow!("Failed to load OIDC key set")));

    let (app, client, cookie) = TestApp::full()
        .with_oidc_keystore(GITHUB_ISSUER_URL, mock_key_store)
        .with_user()
        .await;

    let mut conn = app.db_conn().await;

    let owner_id = cookie.as_model().id;
    let krate = CrateBuilder::new("foo", owner_id).build(&mut conn).await?;
    new_oidc_config(krate.id).insert(&mut conn).await?;

    let body = default_claims().as_exchange_body()?;
    let response = client.put::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"500 Internal Server Error");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"Failed to load OIDC key set"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_audience() -> anyhow::Result<()> {
    let client = prepare().await?;

    let mut claims = default_claims();
    claims.aud = "invalid-audience".into();

    let body = claims.as_exchange_body()?;
    let response = client.put::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"Failed to decode JWT"}]}"#);

    Ok(())
}

/// Test that OIDC tokens can only be exchanged once
#[tokio::test(flavor = "multi_thread")]
async fn test_token_reuse() -> anyhow::Result<()> {
    let client = prepare().await?;

    let body = default_claims().as_exchange_body()?;

    // The first exchange should succeed
    let response = client.put::<()>(URL, body.clone()).await;
    assert_snapshot!(response.status(), @"200 OK");

    // The second exchange should fail
    let response = client.put::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"JWT has already been used"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_repository() -> anyhow::Result<()> {
    let client = prepare().await?;

    let mut claims = default_claims();
    claims.repository = "what?".into();

    let body = claims.as_exchange_body()?;
    let response = client.put::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"Unexpected `repository` value"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_workflow() -> anyhow::Result<()> {
    let client = prepare().await?;

    let mut claims = default_claims();
    claims.workflow_ref = "what?".into();

    let body = claims.as_exchange_body()?;
    let response = client.put::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"Unexpected `workflow_ref` value"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_owner_id() -> anyhow::Result<()> {
    let client = prepare().await?;

    let mut claims = default_claims();
    claims.repository_owner_id = "what?".into();

    let body = claims.as_exchange_body()?;
    let response = client.put::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"Unexpected `repository_owner_id` value"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_missing_config() -> anyhow::Result<()> {
    let (_app, client, _cookie) = TestApp::full()
        .with_oidc_keystore(GITHUB_ISSUER_URL, MockOidcKeyStore::with_test_key())
        .with_user()
        .await;

    let body = default_claims().as_exchange_body()?;
    let response = client.put::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"No matching Trusted Publishing config found"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_missing_environment() -> anyhow::Result<()> {
    let client = prepare_with_config(|c| c.environment = Some("prod")).await?;

    let body = default_claims().as_exchange_body()?;
    let response = client.put::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"No matching Trusted Publishing config found"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_wrong_environment() -> anyhow::Result<()> {
    let client = prepare_with_config(|c| c.environment = Some("prod")).await?;

    let mut claims = default_claims();
    claims.environment = Some("not-prod".into());

    let body = claims.as_exchange_body()?;
    let response = client.put::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"No matching Trusted Publishing config found"}]}"#);

    Ok(())
}

/// Check that the owner name, repository name, and environment are accepted in
/// a case-insensitive manner.
#[tokio::test(flavor = "multi_thread")]
async fn test_case_insensitive() -> anyhow::Result<()> {
    let client = prepare_with_config(|c| c.environment = Some("Prod")).await?;

    let claims = FullGitHubClaims::builder()
        .owner_id(OWNER_ID)
        .owner_name("RUST-lanG")
        .repository_name("foo-RS")
        .workflow_filename(WORKFLOW_FILENAME)
        .environment("PROD")
        .build();

    let body = claims.as_exchange_body()?;
    let response = client.put::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"200 OK");

    Ok(())
}
