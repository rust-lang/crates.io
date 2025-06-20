use crate::tests::builders::{CrateBuilder, PublishBuilder};
use crate::tests::util::{MockTokenUser, RequestHelper, TestApp};
use chrono::{TimeDelta, Utc};
use crates_io_database::models::trustpub::NewToken;
use crates_io_github::{GitHubUser, MockGitHubClient};
use crates_io_trustpub::access_token::AccessToken;
use crates_io_trustpub::github::GITHUB_ISSUER_URL;
use crates_io_trustpub::github::test_helpers::FullGitHubClaims;
use crates_io_trustpub::keystore::MockOidcKeyStore;
use crates_io_trustpub::test_keys::encode_for_testing;
use diesel::QueryResult;
use diesel_async::AsyncPgConnection;
use insta::{assert_json_snapshot, assert_snapshot};
use mockall::predicate::*;
use p256::ecdsa::signature::digest::Output;
use secrecy::ExposeSecret;
use serde_json::json;
use sha2::Sha256;

/// Test the full flow of publishing a crate with OIDC authentication
/// (aka. "Trusted Publishing")
///
/// This test will:
///
/// 1. Publish a new crate via API token.
/// 2. Create a Trusted Publishing configuration.
/// 3. Generate a new OIDC token and exchange it for a temporary access token.
/// 4. Publish a new version of the crate using the temporary access token.
/// 5. Revoke the temporary access token.
#[tokio::test(flavor = "multi_thread")]
async fn test_full_flow() -> anyhow::Result<()> {
    const CRATE_NAME: &str = "foo";

    const OWNER_NAME: &str = "rust-lang";
    const OWNER_ID: i32 = 42;
    const REPOSITORY_NAME: &str = "foo-rs";
    const WORKFLOW_FILENAME: &str = "publish.yml";

    let mut github_mock = MockGitHubClient::new();

    github_mock
        .expect_get_user()
        .with(eq(OWNER_NAME), always())
        .returning(|_, _| {
            Ok(GitHubUser {
                avatar_url: None,
                email: None,
                id: OWNER_ID,
                login: OWNER_NAME.into(),
                name: None,
            })
        });

    let (app, client, cookie_client, api_token_client) = TestApp::full()
        .with_github(github_mock)
        .with_oidc_keystore(GITHUB_ISSUER_URL, MockOidcKeyStore::with_test_key())
        .with_token()
        .await;

    // Step 1: Publish a new crate via API token

    let pb = PublishBuilder::new(CRATE_NAME, "1.0.0");
    let response = api_token_client.publish_crate(pb).await;
    assert_snapshot!(response.status(), @"200 OK");

    // Step 2: Create a Trusted Publishing configuration

    let body = serde_json::to_vec(&json!({
        "github_config": {
            "crate": CRATE_NAME,
            "repository_owner": OWNER_NAME,
            "repository_owner_id": null,
            "repository_name": REPOSITORY_NAME,
            "workflow_filename": WORKFLOW_FILENAME,
            "environment": null,
        }
    }))?;

    let url = "/api/v1/trusted_publishing/github_configs";
    let response = cookie_client.post::<()>(url, body).await;

    assert_json_snapshot!(response.json(), { ".github_config.created_at" => "[datetime]" }, @r#"
    {
      "github_config": {
        "crate": "foo",
        "created_at": "[datetime]",
        "environment": null,
        "id": 1,
        "repository_name": "foo-rs",
        "repository_owner": "rust-lang",
        "repository_owner_id": 42,
        "workflow_filename": "publish.yml"
      }
    }
    "#);

    assert_snapshot!(response.status(), @"200 OK");

    // Step 3: Generate a new OIDC token and exchange it for a temporary access token

    let claims = FullGitHubClaims::builder()
        .owner_id(OWNER_ID)
        .owner_name(OWNER_NAME)
        .repository_name(REPOSITORY_NAME)
        .workflow_filename(WORKFLOW_FILENAME)
        .build();

    let jwt = encode_for_testing(&claims)?;

    let body = serde_json::to_vec(&json!({ "jwt": jwt }))?;
    let response = client
        .post::<()>("/api/v1/trusted_publishing/tokens", body)
        .await;
    let json = response.json();
    assert_json_snapshot!(json, { ".token" => "[token]" }, @r#"
    {
      "token": "[token]"
    }
    "#);
    assert_snapshot!(response.status(), @"200 OK");
    let token = json["token"].as_str().unwrap_or_default();

    // Step 4: Publish a new version of the crate using the temporary access token

    let oidc_token_client = MockTokenUser::with_auth_header(token.to_string(), app.clone());

    let pb = PublishBuilder::new(CRATE_NAME, "1.1.0");
    let response = oidc_token_client.publish_crate(pb).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });

    // Step 4b: Verify the new version was published successfully

    let url = format!("/api/v1/crates/{CRATE_NAME}/1.1.0");
    let response = client.get::<()>(&url).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), {
        ".version.created_at" => "[datetime]",
        ".version.updated_at" => "[datetime]",
        ".version.audit_actions[].time" => "[datetime]",
    });

    // Step 5: Revoke the temporary access token

    let response = oidc_token_client
        .delete::<()>("/api/v1/trusted_publishing/tokens")
        .await;
    assert_snapshot!(response.status(), @"204 No Content");

    assert_snapshot!(app.emails_snapshot().await);

    Ok(())
}

fn generate_token() -> (String, Output<Sha256>) {
    let token = AccessToken::generate();
    (token.finalize().expose_secret().to_string(), token.sha256())
}

async fn new_token(conn: &mut AsyncPgConnection, crate_id: i32) -> QueryResult<String> {
    let (token, hashed_token) = generate_token();

    let new_token = NewToken {
        expires_at: Utc::now() + TimeDelta::minutes(30),
        hashed_token: hashed_token.as_slice(),
        crate_ids: &[crate_id],
    };

    new_token.insert(conn).await?;

    Ok(token)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_happy_path() -> anyhow::Result<()> {
    let (app, _client, cookie_client) = TestApp::full().with_user().await;

    let mut conn = app.db_conn().await;

    let owner_id = cookie_client.as_model().id;
    let krate = CrateBuilder::new("foo", owner_id).build(&mut conn).await?;

    let token = new_token(&mut conn, krate.id).await?;

    let oidc_token_client = MockTokenUser::with_auth_header(token, app);

    let pb = PublishBuilder::new(&krate.name, "1.1.0");
    let response = oidc_token_client.publish_crate(pb).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_happy_path_with_fancy_auth_header() -> anyhow::Result<()> {
    let (app, _client, cookie_client) = TestApp::full().with_user().await;

    let mut conn = app.db_conn().await;

    let owner_id = cookie_client.as_model().id;
    let krate = CrateBuilder::new("foo", owner_id).build(&mut conn).await?;

    let token = new_token(&mut conn, krate.id).await?;

    let header = format!("beaReR     {}", token);
    let oidc_token_client = MockTokenUser::with_auth_header(header, app);

    let pb = PublishBuilder::new(&krate.name, "1.1.0");
    let response = oidc_token_client.publish_crate(pb).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_token_format() -> anyhow::Result<()> {
    let (app, _client, cookie_client) = TestApp::full().with_user().await;

    let mut conn = app.db_conn().await;

    let owner_id = cookie_client.as_model().id;
    let krate = CrateBuilder::new("foo", owner_id).build(&mut conn).await?;

    // Create a client with an invalid authorization header (missing token prefix)
    let header = "invalid-format".to_string();
    let oidc_token_client = MockTokenUser::with_auth_header(header, app);

    let pb = PublishBuilder::new(&krate.name, "1.1.0");
    let response = oidc_token_client.publish_crate(pb).await;
    assert_snapshot!(response.status(), @"401 Unauthorized");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"The given API token does not match the format used by crates.io. Tokens generated before 2020-07-14 were generated with an insecure random number generator, and have been revoked. You can generate a new token at https://crates.io/me. For more information please see https://blog.rust-lang.org/2020/07/14/crates-io-security-advisory.html. We apologize for any inconvenience."}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_bearer_token_format() -> anyhow::Result<()> {
    let (app, _client, cookie_client) = TestApp::full().with_user().await;

    let mut conn = app.db_conn().await;

    let owner_id = cookie_client.as_model().id;
    let krate = CrateBuilder::new("foo", owner_id).build(&mut conn).await?;

    // Create a client with an invalid authorization header (missing token prefix)
    let header = "Bearer invalid-token".to_string();
    let oidc_token_client = MockTokenUser::with_auth_header(header, app);

    let pb = PublishBuilder::new(&krate.name, "1.1.0");
    let response = oidc_token_client.publish_crate(pb).await;
    assert_snapshot!(response.status(), @"401 Unauthorized");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"The given API token does not match the format used by crates.io. Tokens generated before 2020-07-14 were generated with an insecure random number generator, and have been revoked. You can generate a new token at https://crates.io/me. For more information please see https://blog.rust-lang.org/2020/07/14/crates-io-security-advisory.html. We apologize for any inconvenience."}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_non_existent_token() -> anyhow::Result<()> {
    let (app, _client, cookie_client) = TestApp::full().with_user().await;

    let mut conn = app.db_conn().await;

    let owner_id = cookie_client.as_model().id;
    let krate = CrateBuilder::new("foo", owner_id).build(&mut conn).await?;

    // Generate a valid token format, but it doesn't exist in the database
    let (token, _) = generate_token();
    let oidc_token_client = MockTokenUser::with_auth_header(token, app);

    let pb = PublishBuilder::new(&krate.name, "1.1.0");
    let response = oidc_token_client.publish_crate(pb).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Invalid authentication token"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_non_existent_token_with_new_crate() -> anyhow::Result<()> {
    let (app, _client, _cookie_client) = TestApp::full().with_user().await;

    // Generate a valid token format, but it doesn't exist in the database
    let (token, _) = generate_token();
    let oidc_token_client = MockTokenUser::with_auth_header(token, app);

    let pb = PublishBuilder::new("foo", "1.0.0");
    let response = oidc_token_client.publish_crate(pb).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Trusted Publishing tokens do not support creating new crates. Publish the crate manually, first"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_token_for_wrong_crate() -> anyhow::Result<()> {
    let (app, _client, cookie_client) = TestApp::full().with_user().await;

    let mut conn = app.db_conn().await;

    let owner_id = cookie_client.as_model().id;
    let krate = CrateBuilder::new("foo", owner_id).build(&mut conn).await?;
    let token = new_token(&mut conn, krate.id).await?;

    let oidc_token_client = MockTokenUser::with_auth_header(token, app);

    let krate = CrateBuilder::new("bar", owner_id).build(&mut conn).await?;

    let pb = PublishBuilder::new(&krate.name, "1.1.0");
    let response = oidc_token_client.publish_crate(pb).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"The provided access token is not valid for crate `bar`"}]}"#);

    Ok(())
}
