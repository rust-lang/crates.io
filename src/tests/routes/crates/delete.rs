use crate::controllers::krate::delete::{DOWNLOADS_PER_MONTH_LIMIT, DeleteQueryParams};
use crate::models::OwnerKind;
use crate::schema::{crate_downloads, crates};
use crate::tests::builders::{DependencyBuilder, PublishBuilder};
use crate::tests::util::{RequestHelper, Response, TestApp};
use axum::RequestPartsExt;
use bigdecimal::ToPrimitive;
use chrono::{TimeDelta, Utc};
use claims::{assert_none, assert_some};
use crates_io_database::schema::crate_owners;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use http::{Request, StatusCode};
use insta::assert_snapshot;
use serde_json::json;

#[tokio::test]
async fn test_query_params() -> anyhow::Result<()> {
    let check = async |uri| {
        let request = Request::builder().uri(uri).body(())?;
        let (mut parts, _) = request.into_parts();
        Ok::<_, anyhow::Error>(parts.extract::<DeleteQueryParams>().await?)
    };

    let params = check("/api/v1/crates/foo").await?;
    assert_none!(params.message());

    let params = check("/api/v1/crates/foo?").await?;
    assert_none!(params.message());

    let params = check("/api/v1/crates/foo?message=").await?;
    assert_none!(params.message());

    let params = check("/api/v1/crates/foo?message=hello%20world").await?;
    assert_eq!(assert_some!(params.message()), "hello world");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_happy_path_new_crate() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;
    let upstream = app.upstream_index();

    publish_crate(&user, "foo").await;
    let crate_id = adjust_creation_date(&mut conn, "foo", 71).await?;

    // Update downloads count so that it wouldn't be deletable if it wasn't new
    adjust_downloads(&mut conn, crate_id, DOWNLOADS_PER_MONTH_LIMIT * 100).await?;

    assert_crate_exists(&anon, "foo", true).await;
    assert!(upstream.crate_exists("foo")?);
    assert_snapshot!(app.stored_files().await.join("\n"), @r"
    crates/foo/foo-1.0.0.crate
    index/3/f/foo
    rss/crates.xml
    rss/crates/foo.xml
    rss/updates.xml
    ");

    let response = delete_crate(&user, "foo").await;
    assert_snapshot!(response.status(), @"204 No Content");
    assert!(response.body().is_empty());

    assert_snapshot!(app.emails_snapshot().await);

    // Assert that the crate no longer exists
    assert_crate_exists(&anon, "foo", false).await;
    assert!(!upstream.crate_exists("foo")?);
    assert_snapshot!(app.stored_files().await.join("\n"), @r"
    rss/crates.xml
    rss/updates.xml
    ");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_happy_path_old_crate() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;
    let upstream = app.upstream_index();

    publish_crate(&user, "foo").await;
    let crate_id = adjust_creation_date(&mut conn, "foo", 73).await?;
    adjust_downloads(&mut conn, crate_id, DOWNLOADS_PER_MONTH_LIMIT).await?;

    assert_crate_exists(&anon, "foo", true).await;
    assert!(upstream.crate_exists("foo")?);
    assert_snapshot!(app.stored_files().await.join("\n"), @r"
    crates/foo/foo-1.0.0.crate
    index/3/f/foo
    rss/crates.xml
    rss/crates/foo.xml
    rss/updates.xml
    ");

    let response = delete_crate(&user, "foo").await;
    assert_snapshot!(response.status(), @"204 No Content");
    assert!(response.body().is_empty());

    assert_snapshot!(app.emails_snapshot().await);

    // Assert that the crate no longer exists
    assert_crate_exists(&anon, "foo", false).await;
    assert!(!upstream.crate_exists("foo")?);
    assert_snapshot!(app.stored_files().await.join("\n"), @r"
    rss/crates.xml
    rss/updates.xml
    ");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_happy_path_really_old_crate() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;
    let upstream = app.upstream_index();

    publish_crate(&user, "foo").await;
    let crate_id = adjust_creation_date(&mut conn, "foo", 1000 * 24).await?;
    adjust_downloads(&mut conn, crate_id, 30 * DOWNLOADS_PER_MONTH_LIMIT).await?;

    assert_crate_exists(&anon, "foo", true).await;
    assert!(upstream.crate_exists("foo")?);
    assert_snapshot!(app.stored_files().await.join("\n"), @r"
    crates/foo/foo-1.0.0.crate
    index/3/f/foo
    rss/crates.xml
    rss/crates/foo.xml
    rss/updates.xml
    ");

    let response = delete_crate(&user, "foo").await;
    assert_snapshot!(response.status(), @"204 No Content");
    assert!(response.body().is_empty());

    assert_snapshot!(app.emails_snapshot().await);

    // Assert that the crate no longer exists
    assert_crate_exists(&anon, "foo", false).await;
    assert!(!upstream.crate_exists("foo")?);
    assert_snapshot!(app.stored_files().await.join("\n"), @r"
    rss/crates.xml
    rss/updates.xml
    ");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_no_auth() -> anyhow::Result<()> {
    let (_app, anon, user) = TestApp::full().with_user().await;

    publish_crate(&user, "foo").await;

    let response = delete_crate(&anon, "foo").await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);

    assert_crate_exists(&anon, "foo", true).await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_token_auth() -> anyhow::Result<()> {
    let (_app, anon, user, token) = TestApp::full().with_token().await;

    publish_crate(&user, "foo").await;

    let response = delete_crate(&token, "foo").await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action can only be performed on the crates.io website"}]}"#);

    assert_crate_exists(&anon, "foo", true).await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_missing_crate() -> anyhow::Result<()> {
    let (_app, _anon, user) = TestApp::full().with_user().await;

    let response = delete_crate(&user, "foo").await;
    assert_snapshot!(response.status(), @"404 Not Found");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate `foo` does not exist"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_not_owner() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::full().with_user().await;
    let user2 = app.db_new_user("bar").await;

    publish_crate(&user, "foo").await;

    let response = delete_crate(&user2, "foo").await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"only owners have permission to delete crates"}]}"#);

    assert_crate_exists(&anon, "foo", true).await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_team_owner() -> anyhow::Result<()> {
    let (app, anon) = TestApp::full().empty().await;
    let user = app.db_new_user("user-org-owner").await;
    let user2 = app.db_new_user("user-one-team").await;

    publish_crate(&user, "foo").await;

    // Add team owner
    let body = json!({ "owners": ["github:test-org:all"] }).to_string();
    let response = user.put::<()>("/api/v1/crates/foo/owners", body).await;
    assert_snapshot!(response.status(), @"200 OK");

    let response = delete_crate(&user2, "foo").await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"team members don't have permission to delete crates"}]}"#);

    assert_crate_exists(&anon, "foo", true).await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_too_many_owners() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;
    let user2 = app.db_new_user("bar").await;

    publish_crate(&user, "foo").await;
    let crate_id = adjust_creation_date(&mut conn, "foo", 73).await?;

    // Add another owner
    diesel::insert_into(crate_owners::table)
        .values((
            crate_owners::crate_id.eq(crate_id),
            crate_owners::owner_id.eq(user2.as_model().id),
            crate_owners::owner_kind.eq(OwnerKind::User),
        ))
        .execute(&mut conn)
        .await?;

    let response = delete_crate(&user, "foo").await;
    assert_snapshot!(response.status(), @"422 Unprocessable Entity");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"only crates with a single owner can be deleted after 72 hours"}]}"#);

    assert_crate_exists(&anon, "foo", true).await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_too_many_downloads() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    publish_crate(&user, "foo").await;
    let crate_id = adjust_creation_date(&mut conn, "foo", 73).await?;
    adjust_downloads(&mut conn, crate_id, DOWNLOADS_PER_MONTH_LIMIT + 1).await?;

    let response = delete_crate(&user, "foo").await;
    assert_snapshot!(response.status(), @"422 Unprocessable Entity");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"only crates with less than 1000 downloads per month can be deleted after 72 hours"}]}"#);

    assert_crate_exists(&anon, "foo", true).await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_rev_deps() -> anyhow::Result<()> {
    let (_app, anon, user) = TestApp::full().with_user().await;

    publish_crate(&user, "foo").await;

    // Publish another crate
    let pb = PublishBuilder::new("bar", "1.0.0").dependency(DependencyBuilder::new("foo"));
    let response = user.publish_crate(pb).await;
    assert_snapshot!(response.status(), @"200 OK");

    let response = delete_crate(&user, "foo").await;
    assert_snapshot!(response.status(), @"422 Unprocessable Entity");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"only crates without reverse dependencies can be deleted"}]}"#);

    assert_crate_exists(&anon, "foo", true).await;

    Ok(())
}

// Publishes a crate with the given name and a single `v1.0.0` version.
async fn publish_crate(user: &impl RequestHelper, name: &str) {
    let pb = PublishBuilder::new(name, "1.0.0");
    let response = user.publish_crate(pb).await;
    assert_eq!(response.status(), StatusCode::OK);
}

/// Moves the `created_at` field of a crate by the given number of hours
/// into the past and returns the ID of the crate.
async fn adjust_creation_date(
    conn: &mut AsyncPgConnection,
    name: &str,
    hours: i64,
) -> QueryResult<i32> {
    let created_at = Utc::now() - TimeDelta::hours(hours);
    let created_at = created_at.naive_utc();

    diesel::update(crates::table)
        .filter(crates::name.eq(name))
        .set(crates::created_at.eq(created_at))
        .returning(crates::id)
        .get_result(conn)
        .await
}

// Updates the download count of a crate.
async fn adjust_downloads(
    conn: &mut AsyncPgConnection,
    crate_id: i32,
    downloads: u64,
) -> QueryResult<()> {
    let downloads = downloads.to_i64().unwrap_or(i64::MAX);

    diesel::update(crate_downloads::table)
        .filter(crate_downloads::crate_id.eq(crate_id))
        .set(crate_downloads::downloads.eq(downloads))
        .execute(conn)
        .await?;

    Ok(())
}

// Performs the `DELETE` request to delete the crate, and runs any pending
// background jobs, then returns the response.
async fn delete_crate(user: &impl RequestHelper, name: &str) -> Response<()> {
    let url = format!("/api/v1/crates/{name}");
    let response = user.delete::<()>(&url).await;
    user.app().run_pending_background_jobs().await;
    response
}

// Asserts that the crate with the given name exists or not.
async fn assert_crate_exists(user: &impl RequestHelper, name: &str, exists: bool) {
    let expected_status = match exists {
        true => StatusCode::OK,
        false => StatusCode::NOT_FOUND,
    };

    let url = format!("/api/v1/crates/{name}");
    let response = user.get::<()>(&url).await;
    assert_eq!(response.status(), expected_status);
}
