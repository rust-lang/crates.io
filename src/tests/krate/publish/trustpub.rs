use crate::tests::builders::PublishBuilder;
use crate::tests::util::{MockTokenUser, RequestHelper, TestApp};
use crates_io_github::{GitHubUser, MockGitHubClient};
use crates_io_trustpub::github::GITHUB_ISSUER_URL;
use crates_io_trustpub::github::test_helpers::FullGitHubClaims;
use crates_io_trustpub::keystore::MockOidcKeyStore;
use crates_io_trustpub::test_keys::encode_for_testing;
use http::StatusCode;
use insta::assert_json_snapshot;
use mockall::predicate::*;
use serde_json::json;

/// Test the full flow of publishing a crate with OIDC authentication
/// (aka. "Trusted Publishing")
///
/// This test will:
///
/// 1. Publish a new crate via API token.
/// 2. Create a Trusted Publishing configuration.
/// 3. Generate a new OIDC token and exchange it for a temporary access token.
/// 4. Publish a new version of the crate using the temporary access token.
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
    assert_eq!(response.status(), StatusCode::OK);

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
    let response = cookie_client.put::<()>(url, body).await;

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

    assert_eq!(response.status(), StatusCode::OK);

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
        .put::<()>("/api/v1/trusted_publishing/tokens", body)
        .await;
    let json = response.json();
    assert_json_snapshot!(json, { ".token" => "[token]" }, @r#"
    {
      "token": "[token]"
    }
    "#);
    assert_eq!(response.status(), StatusCode::OK);
    let token = json["token"].as_str().unwrap_or_default();

    // Step 4: Publish a new version of the crate using the temporary access token

    let oidc_token_client = MockTokenUser::for_token(token, app);

    let pb = PublishBuilder::new(CRATE_NAME, "1.1.0");
    let response = oidc_token_client.publish_crate(pb).await;
    assert_eq!(response.status(), StatusCode::OK);

    // Step 5: Revoke the temporary access token

    let response = oidc_token_client
        .delete::<()>("/api/v1/trusted_publishing/tokens")
        .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    Ok(())
}
