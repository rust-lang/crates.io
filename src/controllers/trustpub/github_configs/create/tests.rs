use crate::tests::builders::CrateBuilder;
use crate::tests::util::{RequestHelper, Response, TestApp};
use anyhow::anyhow;
use bytes::Bytes;
use crates_io_database::schema::trustpub_configs_github;
use crates_io_github::{GitHubError, GitHubUser, MockGitHubClient};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::StatusCode;
use insta::{assert_json_snapshot, assert_snapshot};
use serde_json::json;

const URL: &str = "/api/v1/trusted_publishing/github_configs";

const CRATE_NAME: &str = "foo";

fn simple_github_mock() -> MockGitHubClient {
    let mut github_mock = MockGitHubClient::new();
    github_mock.expect_get_user().returning(|login, _| {
        Ok(GitHubUser {
            avatar_url: None,
            email: None,
            id: 42,
            login: login.into(),
            name: None,
        })
    });
    github_mock
}

async fn run_test(payload: impl Into<Bytes>) -> (TestApp, Response<()>) {
    async fn inner(payload: Bytes) -> (TestApp, Response<()>) {
        let (app, _client, cookie_client) = TestApp::full()
            .with_github(simple_github_mock())
            .with_user()
            .await;

        let mut conn = app.db_conn().await;

        CrateBuilder::new(CRATE_NAME, cookie_client.as_model().id)
            .build(&mut conn)
            .await
            .unwrap();

        (app, cookie_client.put::<()>(URL, payload).await)
    }

    inner(payload.into()).await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_happy_path() -> anyhow::Result<()> {
    let body = serde_json::to_vec(&json!({
        "github_config": {
            "crate": CRATE_NAME,
            "repository_owner": "rust-lang",
            "repository_name": "foo-rs",
            "workflow_filename": "publish.yml",
            "environment": null,
        }
    }))?;

    let (app, response) = run_test(body).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), { ".github_config.created_at" => "[datetime]" });

    assert_snapshot!(app.emails_snapshot().await);

    let mut conn = app.db_conn().await;
    let config_ids = trustpub_configs_github::table
        .select(trustpub_configs_github::id)
        .get_results::<i32>(&mut conn)
        .await?;

    assert_eq!(config_ids.len(), 1);
    assert_eq!(config_ids[0], 1);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_happy_path_with_environment() -> anyhow::Result<()> {
    let body = serde_json::to_vec(&json!({
        "github_config": {
            "crate": CRATE_NAME,
            "repository_owner": "rust-lang",
            "repository_name": "foo-rs",
            "workflow_filename": "publish.yml",
            "environment": "production",
        }
    }))?;

    let (_app, response) = run_test(body).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), { ".github_config.created_at" => "[datetime]" });

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_empty_body() -> anyhow::Result<()> {
    let (_app, response) = run_test("").await;
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Expected request with `Content-Type: application/json`"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_empty_json_object() -> anyhow::Result<()> {
    let (_app, response) = run_test("{}").await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to deserialize the JSON body into the target type: missing field `github_config` at line 1 column 2"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_owner() -> anyhow::Result<()> {
    let body = serde_json::to_vec(&json!({
        "github_config": {
            "crate": CRATE_NAME,
            "repository_owner": "§$%&",
            "repository_name": "foo-rs",
            "workflow_filename": "publish.yml",
            "environment": null,
        }
    }))?;

    let (_app, response) = run_test(body).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Invalid GitHub repository owner name"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_repo() -> anyhow::Result<()> {
    let body = serde_json::to_vec(&json!({
        "github_config": {
            "crate": CRATE_NAME,
            "repository_owner": "rust-lang",
            "repository_name": "@foo",
            "workflow_filename": "publish.yml",
            "environment": null,
        }
    }))?;

    let (_app, response) = run_test(body).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Invalid GitHub repository name"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_workflow() -> anyhow::Result<()> {
    let body = serde_json::to_vec(&json!({
        "github_config": {
            "crate": CRATE_NAME,
            "repository_owner": "rust-lang",
            "repository_name": "foo-rs",
            "workflow_filename": "workflows/ci.json",
            "environment": null,
        }
    }))?;

    let (_app, response) = run_test(body).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Workflow filename must end with `.yml` or `.yaml`"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_environment() -> anyhow::Result<()> {
    let body = serde_json::to_vec(&json!({
        "github_config": {
            "crate": CRATE_NAME,
            "repository_owner": "rust-lang",
            "repository_name": "foo-rs",
            "workflow_filename": "publish.yml",
            "environment": "",
        }
    }))?;

    let (_app, response) = run_test(body).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Environment name may not be empty (use `null` to omit)"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unauthenticated() -> anyhow::Result<()> {
    let (app, client, cookie_client) = TestApp::full()
        .with_github(simple_github_mock())
        .with_user()
        .await;

    let mut conn = app.db_conn().await;

    CrateBuilder::new(CRATE_NAME, cookie_client.as_model().id)
        .build(&mut conn)
        .await?;

    let body = serde_json::to_vec(&json!({
        "github_config": {
            "crate": CRATE_NAME,
            "repository_owner": "rust-lang",
            "repository_name": "foo-rs",
            "workflow_filename": "publish.yml",
            "environment": null,
        }
    }))?;

    let response = client.put::<()>(URL, body).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_token_auth() -> anyhow::Result<()> {
    let (app, _client, cookie_client, token_client) = TestApp::full()
        .with_github(simple_github_mock())
        .with_token()
        .await;

    let mut conn = app.db_conn().await;

    CrateBuilder::new(CRATE_NAME, cookie_client.as_model().id)
        .build(&mut conn)
        .await?;

    let body = serde_json::to_vec(&json!({
        "github_config": {
            "crate": CRATE_NAME,
            "repository_owner": "rust-lang",
            "repository_name": "foo-rs",
            "workflow_filename": "publish.yml",
            "environment": null,
        }
    }))?;

    let response = token_client.put::<()>(URL, body).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action can only be performed on the crates.io website"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_missing_crate() -> anyhow::Result<()> {
    let (_app, _client, cookie_client) = TestApp::full()
        .with_github(simple_github_mock())
        .with_user()
        .await;

    let body = serde_json::to_vec(&json!({
        "github_config": {
            "crate": CRATE_NAME,
            "repository_owner": "rust-lang",
            "repository_name": "foo-rs",
            "workflow_filename": "publish.yml",
            "environment": null,
        }
    }))?;

    let response = cookie_client.put::<()>(URL, body).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate `foo` does not exist"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_non_owner() -> anyhow::Result<()> {
    let (app, _client, cookie_client) = TestApp::full()
        .with_github(simple_github_mock())
        .with_user()
        .await;

    let mut conn = app.db_conn().await;

    CrateBuilder::new(CRATE_NAME, cookie_client.as_model().id)
        .build(&mut conn)
        .await?;

    let other_client = app.db_new_user("other_user").await;

    let body = serde_json::to_vec(&json!({
        "github_config": {
            "crate": CRATE_NAME,
            "repository_owner": "rust-lang",
            "repository_name": "foo-rs",
            "workflow_filename": "publish.yml",
            "environment": null,
        }
    }))?;

    let response = other_client.put::<()>(URL, body).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"You are not an owner of this crate"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unknown_github_user() -> anyhow::Result<()> {
    let mut github_mock = MockGitHubClient::new();
    github_mock
        .expect_get_user()
        .returning(|login, _| Err(GitHubError::NotFound(anyhow!("User {} not found", login))));

    let (app, _client, cookie_client) = TestApp::full().with_github(github_mock).with_user().await;

    let mut conn = app.db_conn().await;

    CrateBuilder::new(CRATE_NAME, cookie_client.as_model().id)
        .build(&mut conn)
        .await?;

    let body = serde_json::to_vec(&json!({
        "github_config": {
            "crate": CRATE_NAME,
            "repository_owner": "rust-lang",
            "repository_name": "foo-rs",
            "workflow_filename": "publish.yml",
            "environment": null,
        }
    }))?;

    let response = cookie_client.put::<()>(URL, body).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Unknown GitHub user or organization"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_github_error() -> anyhow::Result<()> {
    let mut github_mock = MockGitHubClient::new();
    github_mock
        .expect_get_user()
        .returning(|_, _| Err(GitHubError::Other(anyhow!("Internal Server Error"))));

    let (app, _client, cookie_client) = TestApp::full().with_github(github_mock).with_user().await;

    let mut conn = app.db_conn().await;

    CrateBuilder::new(CRATE_NAME, cookie_client.as_model().id)
        .build(&mut conn)
        .await?;

    let body = serde_json::to_vec(&json!({
        "github_config": {
            "crate": CRATE_NAME,
            "repository_owner": "rust-lang",
            "repository_name": "foo-rs",
            "workflow_filename": "publish.yml",
            "environment": null,
        }
    }))?;

    let response = cookie_client.put::<()>(URL, body).await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Internal Server Error"}]}"#);

    Ok(())
}
