use super::URL;
use crate::builders::CrateBuilder;
use crate::util::{RequestHelper, TestApp};
use crates_io_database::models::token::{CrateScope, EndpointScope};
use crates_io_database::models::trustpub::{GitLabConfig, NewGitLabConfig};
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use insta::{assert_json_snapshot, assert_snapshot};

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
    let (app, _, cookie_client) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let user = cookie_client.as_model();
    let foo = CrateBuilder::new("foo", user.id).build(&mut conn).await?;
    let bar = CrateBuilder::new("bar", user.id).build(&mut conn).await?;

    create_config(&mut conn, foo.id, "foo-rs").await?;
    create_config(&mut conn, bar.id, "BAR").await?;

    let response = cookie_client
        .get_with_query::<()>(URL, &format!("user_id={}", user.id))
        .await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), {
        ".gitlab_configs[].created_at" => "[datetime]",
    });

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unauthorized() -> anyhow::Result<()> {
    let (_, anon_client, _) = TestApp::full().with_user().await;

    let response = anon_client.get_with_query::<()>(URL, "user_id=123").await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_other_user() -> anyhow::Result<()> {
    let (app, _, cookie_client) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let user = cookie_client.as_model();
    let krate = CrateBuilder::new("foo", user.id).build(&mut conn).await?;
    create_config(&mut conn, krate.id, "foo-rs").await?;

    let other_user = app.db_new_user("other").await;
    let response = other_user
        .get_with_query::<()>(URL, &format!("user_id={}", user.id))
        .await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication as the specified user"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_user_with_no_configs() -> anyhow::Result<()> {
    let (app, _, cookie_client) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let user = cookie_client.as_model();
    CrateBuilder::new("foo", user.id).build(&mut conn).await?;

    let response = cookie_client
        .get_with_query::<()>(URL, &format!("user_id={}", user.id))
        .await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"gitlab_configs":[],"meta":{"total":0,"next_page":null}}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_legacy_token_auth() -> anyhow::Result<()> {
    let (app, _, cookie_client, token_client) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;

    let user = cookie_client.as_model();
    let krate = CrateBuilder::new("foo", user.id).build(&mut conn).await?;
    create_config(&mut conn, krate.id, "foo-rs").await?;

    let response = token_client
        .get_with_query::<()>(URL, &format!("user_id={}", user.id))
        .await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"This endpoint cannot be used with legacy API tokens. Use a scoped API token instead."}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_token_auth_without_trusted_publishing_scope() -> anyhow::Result<()> {
    let (app, _, cookie_client, token_client) = TestApp::full()
        .with_scoped_token(None, Some(vec![EndpointScope::PublishUpdate]))
        .await;
    let mut conn = app.db_conn().await;

    let user = cookie_client.as_model();
    let krate = CrateBuilder::new("foo", user.id).build(&mut conn).await?;
    create_config(&mut conn, krate.id, "foo-rs").await?;

    let response = token_client
        .get_with_query::<()>(URL, &format!("user_id={}", user.id))
        .await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this token does not have the required permissions to perform this action"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_token_with_crate_scope_filters_results() -> anyhow::Result<()> {
    let (app, _, cookie_client, token_client) = TestApp::full()
        .with_scoped_token(
            Some(vec![CrateScope::try_from("foo").unwrap()]),
            Some(vec![EndpointScope::TrustedPublishing]),
        )
        .await;
    let mut conn = app.db_conn().await;

    let user = cookie_client.as_model();
    let foo = CrateBuilder::new("foo", user.id).build(&mut conn).await?;
    let bar = CrateBuilder::new("bar", user.id).build(&mut conn).await?;

    create_config(&mut conn, foo.id, "foo-rs").await?;
    create_config(&mut conn, bar.id, "BAR").await?;

    // Token scoped to "foo" should only return foo's config, not bar's
    let response = token_client
        .get_with_query::<()>(URL, &format!("user_id={}", user.id))
        .await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), {
        ".gitlab_configs[].created_at" => "[datetime]",
    });

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_token_without_crate_scope_returns_all() -> anyhow::Result<()> {
    let (app, _, cookie_client, token_client) = TestApp::full()
        .with_scoped_token(None, Some(vec![EndpointScope::TrustedPublishing]))
        .await;
    let mut conn = app.db_conn().await;

    let user = cookie_client.as_model();
    let foo = CrateBuilder::new("foo", user.id).build(&mut conn).await?;
    let bar = CrateBuilder::new("bar", user.id).build(&mut conn).await?;

    create_config(&mut conn, foo.id, "foo-rs").await?;
    create_config(&mut conn, bar.id, "BAR").await?;

    // Token without crate scope should return configs for all user's crates
    let response = token_client
        .get_with_query::<()>(URL, &format!("user_id={}", user.id))
        .await;
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

    let user = cookie_client.as_model();

    // Create 3 crates with 5 configs each (15 total)
    for crate_name in ["aaa", "bbb", "ccc"] {
        let krate = CrateBuilder::new(crate_name, user.id)
            .build(&mut conn)
            .await?;
        for i in 0..5 {
            create_config(&mut conn, krate.id, &format!("{crate_name}-repo-{i}")).await?;
        }
    }

    // Request first page with per_page=5
    let response = cookie_client
        .get_with_query::<()>(URL, &format!("user_id={}&per_page=5", user.id))
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

    // The third page has exactly 5 items, so next_page will be present.
    // Following it should give us an empty fourth page.
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
