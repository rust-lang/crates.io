use crate::tests::builders::CrateBuilder;
use crate::tests::util::{RequestHelper, TestApp};
use crates_io_database::models::trustpub::{GitHubConfig, NewGitHubConfig};
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use http::StatusCode;
use insta::{assert_json_snapshot, assert_snapshot};
use serde_json::json;

const URL: &str = "/api/v1/trusted_publishing/github_configs";

async fn create_config(
    conn: &mut AsyncPgConnection,
    crate_id: i32,
    repository_name: &str,
) -> QueryResult<GitHubConfig> {
    let config = NewGitHubConfig {
        crate_id,
        repository_owner: "rust-lang",
        repository_owner_id: 42,
        repository_name,
        workflow_filename: "publish.yml",
        environment: None,
    };

    config.insert(conn).await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_happy_path() -> anyhow::Result<()> {
    let (app, _client, cookie_client) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let owner_id = cookie_client.as_model().id;
    let foo = CrateBuilder::new("foo", owner_id).build(&mut conn).await?;
    let bar = CrateBuilder::new("bar", owner_id).build(&mut conn).await?;

    create_config(&mut conn, foo.id, "foo-rs").await?;
    create_config(&mut conn, foo.id, "foo").await?;
    create_config(&mut conn, bar.id, "BAR").await?;

    let response = cookie_client.get_with_query::<()>(URL, "crate=foo").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".github_configs[].created_at" => "[datetime]",
    });

    let response = cookie_client.get_with_query::<()>(URL, "crate=Bar").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".github_configs[].created_at" => "[datetime]",
    });

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unauthorized() -> anyhow::Result<()> {
    let (app, anon_client, cookie_client) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let owner_id = cookie_client.as_model().id;
    let krate = CrateBuilder::new("foo", owner_id).build(&mut conn).await?;
    create_config(&mut conn, krate.id, "foo-rs").await?;

    let response = anon_client.get_with_query::<()>(URL, "crate=foo").await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_not_owner() -> anyhow::Result<()> {
    let (app, _, cookie_client) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    // Create a different user who will be the owner of the crate
    let owner_id = cookie_client.as_model().id;
    let krate = CrateBuilder::new("foo", owner_id).build(&mut conn).await?;
    create_config(&mut conn, krate.id, "foo-rs").await?;

    // The authenticated user is not an owner of the crate
    let other_user = app.db_new_user("other").await;
    let response = other_user.get_with_query::<()>(URL, "crate=foo").await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"You are not an owner of this crate"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_team_owner() -> anyhow::Result<()> {
    let (app, _) = TestApp::full().empty().await;
    let mut conn = app.db_conn().await;

    let user = app.db_new_user("user-org-owner").await;
    let user2 = app.db_new_user("user-one-team").await;

    let owner_id = user.as_model().id;
    let krate = CrateBuilder::new("foo", owner_id).build(&mut conn).await?;
    create_config(&mut conn, krate.id, "foo-rs").await?;

    let body = json!({ "owners": ["github:test-org:all"] }).to_string();
    let response = user.put::<()>("/api/v1/crates/foo/owners", body).await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = user2.get_with_query::<()>(URL, "crate=foo").await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"You are not an owner of this crate"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_crate_not_found() -> anyhow::Result<()> {
    let (_, _, cookie_client) = TestApp::full().with_user().await;

    let response = cookie_client.get_with_query::<()>(URL, "crate=foo").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate `foo` does not exist"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_no_query_param() -> anyhow::Result<()> {
    let (_, _, cookie_client) = TestApp::full().with_user().await;

    let response = cookie_client.get::<()>(URL).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to deserialize query string: missing field `crate`"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_crate_with_no_configs() -> anyhow::Result<()> {
    let (app, _, cookie_client) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let owner_id = cookie_client.as_model().id;
    CrateBuilder::new("foo", owner_id).build(&mut conn).await?;

    // No configs have been created for this crate
    let response = cookie_client.get_with_query::<()>(URL, "crate=foo").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".github_configs[].created_at" => "[datetime]",
    });

    Ok(())
}
