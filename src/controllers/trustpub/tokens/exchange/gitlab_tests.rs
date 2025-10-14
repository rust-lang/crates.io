use crate::tests::builders::CrateBuilder;
use crate::tests::util::{MockAnonymousUser, RequestHelper, TestApp};
use claims::{assert_ok, assert_some_eq};
use crates_io_database::models::trustpub::{GitLabConfig, NewGitLabConfig};
use crates_io_database::schema::{trustpub_configs_gitlab, trustpub_tokens};
use crates_io_trustpub::access_token::AccessToken;
use crates_io_trustpub::gitlab::GITLAB_ISSUER_URL;
use crates_io_trustpub::gitlab::test_helpers::FullGitLabClaims;
use crates_io_trustpub::keystore::MockOidcKeyStore;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use insta::{assert_compact_debug_snapshot, assert_json_snapshot, assert_snapshot};
use jsonwebtoken::{EncodingKey, Header};
use mockall::predicate::*;
use serde_json::json;

const URL: &str = "/api/v1/trusted_publishing/tokens";

const CRATE_NAME: &str = "foo";
const NAMESPACE: &str = "rust-lang";
const NAMESPACE_ID: &str = "42";
const PROJECT: &str = "foo-rs";
const WORKFLOW_FILEPATH: &str = "some/subfolder/jobs.yaml";

async fn prepare() -> anyhow::Result<MockAnonymousUser> {
    prepare_with_config(|_config| {}).await
}

async fn prepare_with_config(
    adjust_config: fn(&mut NewGitLabConfig<'static>),
) -> anyhow::Result<MockAnonymousUser> {
    let (app, client, cookie) = TestApp::full()
        .with_oidc_keystore(GITLAB_ISSUER_URL, MockOidcKeyStore::with_test_key())
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

fn new_oidc_config(crate_id: i32) -> NewGitLabConfig<'static> {
    NewGitLabConfig {
        crate_id,
        namespace: NAMESPACE,
        project: PROJECT,
        workflow_filepath: WORKFLOW_FILEPATH,
        environment: None,
    }
}

fn default_claims() -> FullGitLabClaims {
    FullGitLabClaims::builder()
        .namespace_id(NAMESPACE_ID)
        .namespace(NAMESPACE)
        .project(PROJECT)
        .workflow_filepath(WORKFLOW_FILEPATH)
        .build()
}

// ============================================================================
// Success cases
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_happy_path() -> anyhow::Result<()> {
    let client = prepare().await?;

    let body = default_claims().as_exchange_body()?;
    let response = client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"200 OK");

    let json = response.json();
    assert_json_snapshot!(json, { ".token" => "[token]" }, @r#"
    {
      "token": "[token]"
    }
    "#);

    let token = json["token"].as_str().unwrap();
    let token = assert_ok!(token.parse::<AccessToken>());
    let hashed_token = token.sha256();

    let mut conn = client.app().db_conn().await;

    #[expect(deprecated)]
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
    let response = client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"200 OK");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_happy_path_with_ignored_environment() -> anyhow::Result<()> {
    let client = prepare().await?;

    let mut claims = default_claims();
    claims.environment = Some("prod".into());

    let body = claims.as_exchange_body()?;
    let response = client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"200 OK");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_case_insensitive() -> anyhow::Result<()> {
    let client = prepare_with_config(|c| c.environment = Some("Prod")).await?;

    let claims = FullGitLabClaims::builder()
        .namespace_id(NAMESPACE_ID)
        .namespace("RUST-lanG")
        .project("foo-RS")
        .workflow_filepath(WORKFLOW_FILEPATH)
        .environment("PROD")
        .build();

    let body = claims.as_exchange_body()?;
    let response = client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"200 OK");

    Ok(())
}

// ============================================================================
// JWT decode and validation tests
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_broken_jwt() -> anyhow::Result<()> {
    let client = prepare().await?;

    let body = serde_json::to_vec(&json!({ "jwt": "broken" }))?;
    let response = client.post::<()>(URL, body).await;
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
    let response = client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"Unsupported JWT issuer: https://gitlab.com"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_missing_key_id() -> anyhow::Result<()> {
    let client = prepare().await?;

    let claims = default_claims();
    let secret_key = EncodingKey::from_secret(b"secret");
    let jwt = jsonwebtoken::encode(&Header::default(), &claims, &secret_key)?;
    let body = serde_json::to_vec(&json!({ "jwt": jwt }))?;

    let response = client.post::<()>(URL, body).await;
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
        .with_oidc_keystore(GITLAB_ISSUER_URL, mock_key_store)
        .with_user()
        .await;

    let mut conn = app.db_conn().await;

    let owner_id = cookie.as_model().id;
    let krate = CrateBuilder::new("foo", owner_id).build(&mut conn).await?;
    new_oidc_config(krate.id).insert(&mut conn).await?;

    let body = default_claims().as_exchange_body()?;
    let response = client.post::<()>(URL, body).await;
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
        .with_oidc_keystore(GITLAB_ISSUER_URL, mock_key_store)
        .with_user()
        .await;

    let mut conn = app.db_conn().await;

    let owner_id = cookie.as_model().id;
    let krate = CrateBuilder::new("foo", owner_id).build(&mut conn).await?;
    new_oidc_config(krate.id).insert(&mut conn).await?;

    let body = default_claims().as_exchange_body()?;
    let response = client.post::<()>(URL, body).await;
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
    let response = client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"Failed to decode JWT"}]}"#);

    Ok(())
}

// ============================================================================
// JTI replay prevention tests
// ============================================================================

/// Test that OIDC tokens can only be exchanged once
#[tokio::test(flavor = "multi_thread")]
async fn test_token_reuse() -> anyhow::Result<()> {
    let client = prepare().await?;

    let body = default_claims().as_exchange_body()?;

    // The first exchange should succeed
    let response = client.post::<()>(URL, body.clone()).await;
    assert_snapshot!(response.status(), @"200 OK");

    // The second exchange should fail
    let response = client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"JWT has already been used"}]}"#);

    Ok(())
}

// ============================================================================
// Project path parsing tests
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_project_path() -> anyhow::Result<()> {
    let client = prepare().await?;

    let mut claims = default_claims();
    claims.project_path = "what?".into();

    let body = claims.as_exchange_body()?;
    let response = client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"Unexpected `project_path` value"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_project_path_no_slash() -> anyhow::Result<()> {
    let client = prepare().await?;

    let mut claims = default_claims();
    claims.project_path = "invalid-no-slash".into();

    let body = claims.as_exchange_body()?;
    let response = client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"Unexpected `project_path` value"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_subgroup_project_path() -> anyhow::Result<()> {
    let client = prepare_with_config(|c| {
        c.namespace = "group/subgroup";
        c.project = "project";
    })
    .await?;

    let claims = FullGitLabClaims::builder()
        .namespace_id(NAMESPACE_ID)
        .namespace("group/subgroup")
        .project("project")
        .workflow_filepath(WORKFLOW_FILEPATH)
        .build();

    let body = claims.as_exchange_body()?;
    let response = client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"200 OK");

    Ok(())
}

// ============================================================================
// Workflow filepath extraction tests
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_ci_config_ref_uri() -> anyhow::Result<()> {
    let client = prepare().await?;

    let mut claims = default_claims();
    claims.ci_config_ref_uri = "what?".into();

    let body = claims.as_exchange_body()?;
    let response = client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"Unexpected `ci_config_ref_uri` value"}]}"#);

    Ok(())
}

// ============================================================================
// Config lookup tests
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_missing_config() -> anyhow::Result<()> {
    let (_app, client, _cookie) = TestApp::full()
        .with_oidc_keystore(GITLAB_ISSUER_URL, MockOidcKeyStore::with_test_key())
        .with_user()
        .await;

    let body = default_claims().as_exchange_body()?;
    let response = client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"No Trusted Publishing config found for repository `rust-lang/foo-rs`."}]}"#);

    Ok(())
}

// ============================================================================
// Namespace ID lazy population and resurrection protection tests
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_lazy_namespace_id_population() -> anyhow::Result<()> {
    let client = prepare().await?;
    let mut conn = client.app().db_conn().await;

    // First exchange should succeed and populate namespace_id
    let body = default_claims().as_exchange_body()?;
    let response = client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"200 OK");

    // Check that namespace_id was populated in the database
    let config: GitLabConfig = trustpub_configs_gitlab::table
        .filter(trustpub_configs_gitlab::namespace.eq(NAMESPACE))
        .filter(trustpub_configs_gitlab::project.eq(PROJECT))
        .select(GitLabConfig::as_select())
        .first(&mut conn)
        .await?;

    assert_some_eq!(config.namespace_id, NAMESPACE_ID);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_namespace_id_mismatch_resurrection_attack() -> anyhow::Result<()> {
    // Create a config with a pre-populated namespace_id
    let client = prepare().await?;

    let mut conn = client.app().db_conn().await;

    // Manually update the config to have a different namespace_id
    diesel::update(
        trustpub_configs_gitlab::table
            .filter(trustpub_configs_gitlab::namespace.eq(NAMESPACE))
            .filter(trustpub_configs_gitlab::project.eq(PROJECT)),
    )
    .set(trustpub_configs_gitlab::namespace_id.eq("999"))
    .execute(&mut conn)
    .await?;

    // Try to exchange with different namespace_id - should fail
    let body = default_claims().as_exchange_body()?;
    let response = client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"The Trusted Publishing config for repository `rust-lang/foo-rs` does not match the namespace ID (42) in the JWT. Expected namespace IDs: 999. Please recreate the Trusted Publishing config to update the namespace ID."}]}"#);

    Ok(())
}

// ============================================================================
// Workflow filepath matching tests
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_wrong_workflow_filepath() -> anyhow::Result<()> {
    let client = prepare().await?;

    let mut claims = default_claims();
    claims.ci_config_ref_uri =
        "gitlab.com/rust-lang/foo-rs//wrong-workflow.yml@refs/heads/main".into();

    let body = claims.as_exchange_body()?;
    let response = client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"The Trusted Publishing config for repository `rust-lang/foo-rs` does not match the workflow filepath `wrong-workflow.yml` in the JWT. Expected workflow filepaths: `some/subfolder/jobs.yaml`"}]}"#);

    Ok(())
}

// ============================================================================
// Environment matching tests
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_missing_environment() -> anyhow::Result<()> {
    let client = prepare_with_config(|c| c.environment = Some("prod")).await?;

    let body = default_claims().as_exchange_body()?;
    let response = client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"The Trusted Publishing config for repository `rust-lang/foo-rs` requires an environment, but the JWT does not specify one. Expected environments: `prod`"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_wrong_environment() -> anyhow::Result<()> {
    let client = prepare_with_config(|c| c.environment = Some("prod")).await?;

    let mut claims = default_claims();
    claims.environment = Some("not-prod".into());

    let body = claims.as_exchange_body()?;
    let response = client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"The Trusted Publishing config for repository `rust-lang/foo-rs` does not match the environment `not-prod` in the JWT. Expected environments: `prod`"}]}"#);

    Ok(())
}
