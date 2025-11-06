use crate::builders::CrateBuilder;
use crate::util::{RequestHelper, Response, TestApp};
use bytes::Bytes;
use crates_io_database::models::token::{CrateScope, EndpointScope};
use crates_io_database::schema::{emails, trustpub_configs_gitlab};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use insta::{assert_json_snapshot, assert_snapshot};
use serde_json::json;

const URL: &str = "/api/v1/trusted_publishing/gitlab_configs";

const CRATE_NAME: &str = "foo";

async fn run_test(payload: impl Into<Bytes>) -> (TestApp, Response<()>) {
    async fn inner(payload: Bytes) -> (TestApp, Response<()>) {
        let (app, _client, cookie_client) = TestApp::full().with_user().await;

        let mut conn = app.db_conn().await;

        CrateBuilder::new(CRATE_NAME, cookie_client.as_model().id)
            .build(&mut conn)
            .await
            .unwrap();

        (app, cookie_client.post::<()>(URL, payload).await)
    }

    inner(payload.into()).await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_happy_path() -> anyhow::Result<()> {
    let body = serde_json::to_vec(&json!({
        "gitlab_config": {
            "crate": CRATE_NAME,
            "namespace": "rust-lang",
            "project": "foo-rs",
            "workflow_filepath": ".gitlab-ci.yml",
            "environment": null,
        }
    }))?;

    let (app, response) = run_test(body).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), { ".gitlab_config.created_at" => "[datetime]" });

    assert_snapshot!(app.emails_snapshot().await);

    let mut conn = app.db_conn().await;
    let config_ids = trustpub_configs_gitlab::table
        .select(trustpub_configs_gitlab::id)
        .get_results::<i32>(&mut conn)
        .await?;

    assert_eq!(config_ids.len(), 1);
    assert_eq!(config_ids[0], 1);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_happy_path_with_environment() -> anyhow::Result<()> {
    let body = serde_json::to_vec(&json!({
        "gitlab_config": {
            "crate": CRATE_NAME,
            "namespace": "rust-lang",
            "project": "foo-rs",
            "workflow_filepath": ".gitlab-ci.yml",
            "environment": "production",
        }
    }))?;

    let (_app, response) = run_test(body).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), { ".gitlab_config.created_at" => "[datetime]" });

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_empty_body() -> anyhow::Result<()> {
    let (_app, response) = run_test("").await;
    assert_snapshot!(response.status(), @"415 Unsupported Media Type");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Expected request with `Content-Type: application/json`"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_empty_json_object() -> anyhow::Result<()> {
    let (_app, response) = run_test("{}").await;
    assert_snapshot!(response.status(), @"422 Unprocessable Entity");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to deserialize the JSON body into the target type: missing field `gitlab_config` at line 1 column 2"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_namespace() -> anyhow::Result<()> {
    let body = serde_json::to_vec(&json!({
        "gitlab_config": {
            "crate": CRATE_NAME,
            "namespace": "§$%&",
            "project": "foo-rs",
            "workflow_filepath": ".gitlab-ci.yml",
            "environment": null,
        }
    }))?;

    let (_app, response) = run_test(body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Invalid GitLab namespace"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_project() -> anyhow::Result<()> {
    let body = serde_json::to_vec(&json!({
        "gitlab_config": {
            "crate": CRATE_NAME,
            "namespace": "rust-lang",
            "project": "@foo",
            "workflow_filepath": ".gitlab-ci.yml",
            "environment": null,
        }
    }))?;

    let (_app, response) = run_test(body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Invalid GitLab project name"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_workflow_filepath() -> anyhow::Result<()> {
    let body = serde_json::to_vec(&json!({
        "gitlab_config": {
            "crate": CRATE_NAME,
            "namespace": "rust-lang",
            "project": "foo-rs",
            "workflow_filepath": "ci.json",
            "environment": null,
        }
    }))?;

    let (_app, response) = run_test(body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Workflow filepath must end with `.yml` or `.yaml`"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_environment() -> anyhow::Result<()> {
    let body = serde_json::to_vec(&json!({
        "gitlab_config": {
            "crate": CRATE_NAME,
            "namespace": "rust-lang",
            "project": "foo-rs",
            "workflow_filepath": ".gitlab-ci.yml",
            "environment": "",
        }
    }))?;

    let (_app, response) = run_test(body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Environment name may not be empty (use `null` to omit)"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unauthenticated() -> anyhow::Result<()> {
    let (app, client, cookie_client) = TestApp::full().with_user().await;

    let mut conn = app.db_conn().await;

    CrateBuilder::new(CRATE_NAME, cookie_client.as_model().id)
        .build(&mut conn)
        .await?;

    let body = serde_json::to_vec(&json!({
        "gitlab_config": {
            "crate": CRATE_NAME,
            "namespace": "rust-lang",
            "project": "foo-rs",
            "workflow_filepath": ".gitlab-ci.yml",
            "environment": null,
        }
    }))?;

    let response = client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_legacy_token_auth() -> anyhow::Result<()> {
    let (app, _client, cookie_client, token_client) = TestApp::full().with_token().await;

    let mut conn = app.db_conn().await;

    CrateBuilder::new(CRATE_NAME, cookie_client.as_model().id)
        .build(&mut conn)
        .await?;

    let body = serde_json::to_vec(&json!({
        "gitlab_config": {
            "crate": CRATE_NAME,
            "namespace": "rust-lang",
            "project": "foo-rs",
            "workflow_filepath": ".gitlab-ci.yml",
            "environment": null,
        }
    }))?;

    let response = token_client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), { ".gitlab_config.created_at" => "[datetime]" });

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_token_auth_with_trusted_publishing_scope() -> anyhow::Result<()> {
    let (app, _client, cookie_client, token_client) = TestApp::full()
        .with_scoped_token(
            Some(vec![CrateScope::try_from(CRATE_NAME).unwrap()]),
            Some(vec![EndpointScope::TrustedPublishing]),
        )
        .await;

    let mut conn = app.db_conn().await;

    CrateBuilder::new(CRATE_NAME, cookie_client.as_model().id)
        .build(&mut conn)
        .await?;

    let body = serde_json::to_vec(&json!({
        "gitlab_config": {
            "crate": CRATE_NAME,
            "namespace": "rust-lang",
            "project": "foo-rs",
            "workflow_filepath": ".gitlab-ci.yml",
            "environment": null,
        }
    }))?;

    let response = token_client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), { ".gitlab_config.created_at" => "[datetime]" });

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_token_auth_without_trusted_publishing_scope() -> anyhow::Result<()> {
    let (app, _client, cookie_client, token_client) = TestApp::full()
        .with_scoped_token(
            Some(vec![CrateScope::try_from(CRATE_NAME).unwrap()]),
            Some(vec![EndpointScope::PublishUpdate]),
        )
        .await;

    let mut conn = app.db_conn().await;

    CrateBuilder::new(CRATE_NAME, cookie_client.as_model().id)
        .build(&mut conn)
        .await?;

    let body = serde_json::to_vec(&json!({
        "gitlab_config": {
            "crate": CRATE_NAME,
            "namespace": "rust-lang",
            "project": "foo-rs",
            "workflow_filepath": ".gitlab-ci.yml",
            "environment": null,
        }
    }))?;

    let response = token_client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this token does not have the required permissions to perform this action"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_token_auth_with_wrong_crate_scope() -> anyhow::Result<()> {
    let (app, _client, cookie_client, token_client) = TestApp::full()
        .with_scoped_token(
            Some(vec![CrateScope::try_from("other-crate").unwrap()]),
            Some(vec![EndpointScope::TrustedPublishing]),
        )
        .await;

    let mut conn = app.db_conn().await;

    CrateBuilder::new(CRATE_NAME, cookie_client.as_model().id)
        .build(&mut conn)
        .await?;

    let body = serde_json::to_vec(&json!({
        "gitlab_config": {
            "crate": CRATE_NAME,
            "namespace": "rust-lang",
            "project": "foo-rs",
            "workflow_filepath": ".gitlab-ci.yml",
            "environment": null,
        }
    }))?;

    let response = token_client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this token does not have the required permissions to perform this action"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_token_auth_with_wildcard_crate_scope() -> anyhow::Result<()> {
    let (app, _client, cookie_client, token_client) = TestApp::full()
        .with_scoped_token(
            Some(vec![CrateScope::try_from("*").unwrap()]),
            Some(vec![EndpointScope::TrustedPublishing]),
        )
        .await;

    let mut conn = app.db_conn().await;

    CrateBuilder::new(CRATE_NAME, cookie_client.as_model().id)
        .build(&mut conn)
        .await?;

    let body = serde_json::to_vec(&json!({
        "gitlab_config": {
            "crate": CRATE_NAME,
            "namespace": "rust-lang",
            "project": "foo-rs",
            "workflow_filepath": ".gitlab-ci.yml",
            "environment": null,
        }
    }))?;

    let response = token_client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), { ".gitlab_config.created_at" => "[datetime]" });

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_missing_crate() -> anyhow::Result<()> {
    let (_app, _client, cookie_client) = TestApp::full().with_user().await;

    let body = serde_json::to_vec(&json!({
        "gitlab_config": {
            "crate": CRATE_NAME,
            "namespace": "rust-lang",
            "project": "foo-rs",
            "workflow_filepath": ".gitlab-ci.yml",
            "environment": null,
        }
    }))?;

    let response = cookie_client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"404 Not Found");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate `foo` does not exist"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_non_owner() -> anyhow::Result<()> {
    let (app, _client, cookie_client) = TestApp::full().with_user().await;

    let mut conn = app.db_conn().await;

    CrateBuilder::new(CRATE_NAME, cookie_client.as_model().id)
        .build(&mut conn)
        .await?;

    let other_client = app.db_new_user("other_user").await;

    let body = serde_json::to_vec(&json!({
        "gitlab_config": {
            "crate": CRATE_NAME,
            "namespace": "rust-lang",
            "project": "foo-rs",
            "workflow_filepath": ".gitlab-ci.yml",
            "environment": null,
        }
    }))?;

    let response = other_client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"You are not an owner of this crate"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unverified_email() -> anyhow::Result<()> {
    let (app, _client, cookie_client) = TestApp::full().with_user().await;

    let mut conn = app.db_conn().await;

    diesel::update(emails::table.filter(emails::user_id.eq(cookie_client.as_model().id)))
        .set(emails::verified.eq(false))
        .execute(&mut conn)
        .await?;

    CrateBuilder::new(CRATE_NAME, cookie_client.as_model().id)
        .build(&mut conn)
        .await?;

    let body = serde_json::to_vec(&json!({
        "gitlab_config": {
            "crate": CRATE_NAME,
            "namespace": "rust-lang",
            "project": "foo-rs",
            "workflow_filepath": ".gitlab-ci.yml",
            "environment": null,
        }
    }))?;

    let response = cookie_client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"You must verify your email address to create a Trusted Publishing config"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_too_many_configs() -> anyhow::Result<()> {
    let (app, _client, cookie_client) = TestApp::full().with_user().await;

    let mut conn = app.db_conn().await;

    CrateBuilder::new(CRATE_NAME, cookie_client.as_model().id)
        .build(&mut conn)
        .await?;

    // Create 5 configurations (the maximum)
    for i in 0..5 {
        let body = serde_json::to_vec(&json!({
            "gitlab_config": {
                "crate": CRATE_NAME,
                "namespace": "rust-lang",
                "project": format!("foo-rs-{}", i),
                "workflow_filepath": ".gitlab-ci.yml",
                "environment": null,
            }
        }))?;

        let response = cookie_client.post::<()>(URL, body).await;
        assert_eq!(response.status(), 200);
    }

    // Try to create a 6th configuration
    let body = serde_json::to_vec(&json!({
        "gitlab_config": {
            "crate": CRATE_NAME,
            "namespace": "rust-lang",
            "project": "foo-rs-6",
            "workflow_filepath": ".gitlab-ci.yml",
            "environment": null,
        }
    }))?;

    let response = cookie_client.post::<()>(URL, body).await;
    assert_snapshot!(response.status(), @"409 Conflict");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"This crate already has the maximum number of GitLab Trusted Publishing configurations (5)"}]}"#);

    Ok(())
}
