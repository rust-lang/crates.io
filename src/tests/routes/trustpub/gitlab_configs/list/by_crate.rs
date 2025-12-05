use super::URL;
use crate::builders::CrateBuilder;
use crate::util::{RequestHelper, TestApp};
use crates_io_database::models::token::{CrateScope, EndpointScope};
use crates_io_database::models::trustpub::{GitLabConfig, NewGitLabConfig};
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use insta::{assert_json_snapshot, assert_snapshot};
use serde_json::json;

async fn create_config(
    conn: &mut AsyncPgConnection,
    crate_id: i32,
    project: &str,
) -> QueryResult<GitLabConfig> {
    let config = NewGitLabConfig {
        crate_id,
        namespace: "rust-lang",
        project,
        workflow_filepath: ".gitlab-ci.yml",
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
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), {
        ".gitlab_configs[].created_at" => "[datetime]",
    });

    let response = cookie_client.get_with_query::<()>(URL, "crate=Bar").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), {
        ".gitlab_configs[].created_at" => "[datetime]",
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
    assert_snapshot!(response.status(), @"403 Forbidden");
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
    assert_snapshot!(response.status(), @"400 Bad Request");
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
    assert_snapshot!(response.status(), @"200 OK");

    let response = user2.get_with_query::<()>(URL, "crate=foo").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"You are not an owner of this crate"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_crate_not_found() -> anyhow::Result<()> {
    let (_, _, cookie_client) = TestApp::full().with_user().await;

    let response = cookie_client.get_with_query::<()>(URL, "crate=foo").await;
    assert_snapshot!(response.status(), @"404 Not Found");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate `foo` does not exist"}]}"#);

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
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), {
        ".gitlab_configs[].created_at" => "[datetime]",
    });

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_legacy_token_auth() -> anyhow::Result<()> {
    let (app, _client, cookie_client, token_client) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;

    let owner_id = cookie_client.as_model().id;
    let krate = CrateBuilder::new("foo", owner_id).build(&mut conn).await?;
    create_config(&mut conn, krate.id, "foo-rs").await?;

    let response = token_client.get_with_query::<()>(URL, "crate=foo").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), {
        ".gitlab_configs[].created_at" => "[datetime]",
    });

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_token_auth_with_trusted_publishing_scope() -> anyhow::Result<()> {
    let (app, _client, cookie_client, token_client) = TestApp::full()
        .with_scoped_token(
            Some(vec![CrateScope::try_from("foo").unwrap()]),
            Some(vec![EndpointScope::TrustedPublishing]),
        )
        .await;
    let mut conn = app.db_conn().await;

    let owner_id = cookie_client.as_model().id;
    let krate = CrateBuilder::new("foo", owner_id).build(&mut conn).await?;
    create_config(&mut conn, krate.id, "foo-rs").await?;

    let response = token_client.get_with_query::<()>(URL, "crate=foo").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), {
        ".gitlab_configs[].created_at" => "[datetime]",
    });

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_token_auth_without_trusted_publishing_scope() -> anyhow::Result<()> {
    let (app, _client, cookie_client, token_client) = TestApp::full()
        .with_scoped_token(
            Some(vec![CrateScope::try_from("foo").unwrap()]),
            Some(vec![EndpointScope::PublishUpdate]),
        )
        .await;
    let mut conn = app.db_conn().await;

    let owner_id = cookie_client.as_model().id;
    let krate = CrateBuilder::new("foo", owner_id).build(&mut conn).await?;
    create_config(&mut conn, krate.id, "foo-rs").await?;

    let response = token_client.get_with_query::<()>(URL, "crate=foo").await;
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

    let owner_id = cookie_client.as_model().id;
    let krate = CrateBuilder::new("foo", owner_id).build(&mut conn).await?;
    create_config(&mut conn, krate.id, "foo-rs").await?;

    let response = token_client.get_with_query::<()>(URL, "crate=foo").await;
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

    let owner_id = cookie_client.as_model().id;
    let krate = CrateBuilder::new("foo", owner_id).build(&mut conn).await?;
    create_config(&mut conn, krate.id, "foo-rs").await?;

    let response = token_client.get_with_query::<()>(URL, "crate=foo").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), {
        ".gitlab_configs[].created_at" => "[datetime]",
    });

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_pagination() -> anyhow::Result<()> {
    let (app, _, cookie_client) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let owner_id = cookie_client.as_model().id;
    let krate = CrateBuilder::new("foo", owner_id).build(&mut conn).await?;

    // Create 15 configs
    for i in 0..15 {
        create_config(&mut conn, krate.id, &format!("repo-{i}")).await?;
    }

    // Request first page with per_page=5
    let response = cookie_client
        .get_with_query::<()>(URL, "crate=foo&per_page=5")
        .await;
    assert_snapshot!(response.status(), @"200 OK");
    let json = response.json();
    assert_json_snapshot!(json, {
        ".gitlab_configs[].created_at" => "[datetime]",
    });

    // Extract the next_page URL and make a second request
    let next_page = json["meta"]["next_page"]
        .as_str()
        .expect("next_page should be present");
    let next_url = format!("{URL}{next_page}");
    let response = cookie_client.get::<()>(&next_url).await;
    assert_snapshot!(response.status(), @"200 OK");
    let json = response.json();
    assert_json_snapshot!(json, {
        ".gitlab_configs[].created_at" => "[datetime]",
    });

    // Third page (last page with data)
    let next_page = json["meta"]["next_page"]
        .as_str()
        .expect("next_page should be present");
    let next_url = format!("{URL}{next_page}");
    let response = cookie_client.get::<()>(&next_url).await;
    assert_snapshot!(response.status(), @"200 OK");
    let json = response.json();
    assert_json_snapshot!(json, {
        ".gitlab_configs[].created_at" => "[datetime]",
    });

    // The third page has exactly 5 items, so next_page will be present
    // (cursor-based pagination is conservative about indicating more pages)
    // Following it should give us an empty fourth page
    let next_page = json["meta"]["next_page"]
        .as_str()
        .expect("next_page should be present on third page");
    let next_url = format!("{URL}{next_page}");
    let response = cookie_client.get::<()>(&next_url).await;
    assert_snapshot!(response.status(), @"200 OK");
    let json = response.json();
    assert_json_snapshot!(json, {
        ".gitlab_configs[].created_at" => "[datetime]",
    });

    Ok(())
}
