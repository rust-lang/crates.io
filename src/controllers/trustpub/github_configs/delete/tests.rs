use crate::tests::builders::CrateBuilder;
use crate::tests::util::{RequestHelper, TestApp};
use crates_io_database::models::Crate;
use crates_io_database::models::trustpub::{GitHubConfig, NewGitHubConfig};
use crates_io_database::schema::trustpub_configs_github;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use http::StatusCode;
use insta::assert_snapshot;
use serde_json::json;

const BASE_URL: &str = "/api/v1/trusted_publishing/github_configs";
const CRATE_NAME: &str = "foo";

fn delete_url(id: i32) -> String {
    format!("{BASE_URL}/{id}")
}

async fn create_crate(conn: &mut AsyncPgConnection, author_id: i32) -> anyhow::Result<Crate> {
    CrateBuilder::new(CRATE_NAME, author_id).build(conn).await
}

async fn create_config(conn: &mut AsyncPgConnection, crate_id: i32) -> QueryResult<GitHubConfig> {
    let config = NewGitHubConfig {
        crate_id,
        repository_owner: "rust-lang",
        repository_owner_id: 42,
        repository_name: "foo-rs",
        workflow_filename: "publish.yml",
        environment: None,
    };

    config.insert(conn).await
}

async fn get_all_configs(conn: &mut AsyncPgConnection) -> QueryResult<Vec<GitHubConfig>> {
    trustpub_configs_github::table
        .select(GitHubConfig::as_select())
        .load::<GitHubConfig>(conn)
        .await
}

/// Delete the config with a valid user that is an owner of the crate.
#[tokio::test(flavor = "multi_thread")]
async fn test_happy_path() -> anyhow::Result<()> {
    let (app, _client, cookie_client) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let krate = create_crate(&mut conn, cookie_client.as_model().id).await?;
    let config = create_config(&mut conn, krate.id).await?;

    let response = cookie_client.delete::<()>(&delete_url(config.id)).await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(response.text(), "");

    // Verify the config was deleted from the database
    let configs = get_all_configs(&mut conn).await?;
    assert_eq!(configs.len(), 0);

    // Verify emails were sent to crate owners
    assert_snapshot!(app.emails_snapshot().await);

    Ok(())
}

/// Try to delete the config with an unauthenticated client.
#[tokio::test(flavor = "multi_thread")]
async fn test_unauthenticated() -> anyhow::Result<()> {
    let (app, client, cookie_client) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let krate = create_crate(&mut conn, cookie_client.as_model().id).await?;
    let config = create_config(&mut conn, krate.id).await?;

    let response = client.delete::<()>(&delete_url(config.id)).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);

    // Verify the config was not deleted
    let configs = get_all_configs(&mut conn).await?;
    assert_eq!(configs.len(), 1);

    // Verify no emails were sent to crate owners
    assert_eq!(app.emails().await.len(), 0);

    Ok(())
}

/// Try to delete the config with API token authentication.
#[tokio::test(flavor = "multi_thread")]
async fn test_token_auth() -> anyhow::Result<()> {
    let (app, _client, cookie_client, token_client) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;

    let krate = create_crate(&mut conn, cookie_client.as_model().id).await?;
    let config = create_config(&mut conn, krate.id).await?;

    let response = token_client.delete::<()>(&delete_url(config.id)).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action can only be performed on the crates.io website"}]}"#);

    // Verify the config was not deleted
    let configs = get_all_configs(&mut conn).await?;
    assert_eq!(configs.len(), 1);

    // Verify no emails were sent to crate owners
    assert_eq!(app.emails().await.len(), 0);

    Ok(())
}

/// Try to delete a config that does not exist.
#[tokio::test(flavor = "multi_thread")]
async fn test_config_not_found() -> anyhow::Result<()> {
    let (app, _client, cookie_client) = TestApp::full().with_user().await;

    let response = cookie_client.delete::<()>(&delete_url(42)).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Not Found"}]}"#);

    // Verify no emails were sent to crate owners
    assert_eq!(app.emails().await.len(), 0);

    Ok(())
}

/// Try to delete the config with a user who is not an owner of the crate.
#[tokio::test(flavor = "multi_thread")]
async fn test_non_owner() -> anyhow::Result<()> {
    let (app, _client, cookie_client) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let krate = create_crate(&mut conn, cookie_client.as_model().id).await?;
    let config = create_config(&mut conn, krate.id).await?;

    // Create another user who is not an owner of the crate
    let other_client = app.db_new_user("other_user").await;

    let response = other_client.delete::<()>(&delete_url(config.id)).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"You are not an owner of this crate"}]}"#);

    // Verify the config was not deleted
    let configs = get_all_configs(&mut conn).await?;
    assert_eq!(configs.len(), 1);

    // Verify no emails were sent to crate owners
    assert_eq!(app.emails().await.len(), 0);

    Ok(())
}

/// Try to delete the config with a user that is part of a team that owns
/// the crate.
#[tokio::test(flavor = "multi_thread")]
async fn test_team_owner() -> anyhow::Result<()> {
    let (app, _client) = TestApp::full().empty().await;
    let mut conn = app.db_conn().await;

    let user = app.db_new_user("user-org-owner").await;
    let user2 = app.db_new_user("user-one-team").await;

    let krate = create_crate(&mut conn, user.as_model().id).await?;
    let config = create_config(&mut conn, krate.id).await?;

    let body = json!({ "owners": ["github:test-org:all"] }).to_string();
    let response = user.put::<()>("/api/v1/crates/foo/owners", body).await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = user2.delete::<()>(&delete_url(config.id)).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"You are not an owner of this crate"}]}"#);

    // Verify the config was not deleted
    let configs = get_all_configs(&mut conn).await?;
    assert_eq!(configs.len(), 1);

    // Verify no emails were sent to crate owners
    assert_eq!(app.emails().await.len(), 0);

    Ok(())
}
