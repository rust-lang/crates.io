use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::models::{Crate, NewDeletedCrate, Rights};
use crate::schema::{crate_downloads, crates, dependencies};
use crate::util::errors::{crate_not_found, custom, AppResult, BoxedAppError};
use crate::worker::jobs;
use axum::extract::Path;
use bigdecimal::ToPrimitive;
use chrono::{TimeDelta, Utc};
use crates_io_database::schema::deleted_crates;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use http::request::Parts;
use http::StatusCode;

const DOWNLOADS_PER_MONTH_LIMIT: u64 = 100;
const AVAILABLE_AFTER: TimeDelta = TimeDelta::hours(24);

/// Deletes a crate from the database, index and storage.
///
/// The crate can only be deleted by the owner of the crate, and only if the
/// crate has been published for less than 72 hours, or if the crate has a
/// single owner, has been downloaded less than 100 times for each month it has
/// been published, and is not depended upon by any other crate on crates.io.
pub async fn delete(
    Path(name): Path<String>,
    parts: Parts,
    app: AppState,
) -> AppResult<StatusCode> {
    let mut conn = app.db_write().await?;

    // Check that the user is authenticated
    let auth = AuthCheck::only_cookie().check(&parts, &mut conn).await?;

    // Check that the crate exists
    let krate = find_crate(&mut conn, &name).await?;
    let krate = krate.ok_or_else(|| crate_not_found(&name))?;

    // Check that the user is an owner of the crate (team owners are not allowed to delete crates)
    let user = auth.user();
    let owners = krate.owners(&mut conn).await?;
    match user.rights(&app, &owners).await? {
        Rights::Full => {}
        Rights::Publish => {
            let msg = "team members don't have permission to delete crates";
            return Err(custom(StatusCode::FORBIDDEN, msg));
        }
        Rights::None => {
            let msg = "only owners have permission to delete crates";
            return Err(custom(StatusCode::FORBIDDEN, msg));
        }
    }

    let created_at = krate.created_at.and_utc();

    let age = Utc::now().signed_duration_since(created_at);
    if age > TimeDelta::hours(72) {
        if owners.len() > 1 {
            let msg = "only crates with a single owner can be deleted after 72 hours";
            return Err(custom(StatusCode::UNPROCESSABLE_ENTITY, msg));
        }

        let downloads = get_crate_downloads(&mut conn, krate.id).await?;
        if downloads > max_downloads(&age) {
            let msg = format!("only crates with less than {DOWNLOADS_PER_MONTH_LIMIT} downloads per month can be deleted after 72 hours");
            return Err(custom(StatusCode::UNPROCESSABLE_ENTITY, msg));
        }

        if has_rev_dep(&mut conn, krate.id).await? {
            let msg = "only crates without reverse dependencies can be deleted after 72 hours";
            return Err(custom(StatusCode::UNPROCESSABLE_ENTITY, msg));
        }
    }

    conn.transaction(|conn| {
        async move {
            diesel::delete(crates::table.find(krate.id))
                .execute(conn)
                .await?;

            let deleted_at = Utc::now();
            let available_at = deleted_at + AVAILABLE_AFTER;

            let deleted_crate = NewDeletedCrate::builder(&krate.name)
                .created_at(&created_at)
                .deleted_at(&deleted_at)
                .deleted_by(user.id)
                .available_at(&available_at)
                .build();

            diesel::insert_into(deleted_crates::table)
                .values(deleted_crate)
                .execute(conn)
                .await?;

            let git_index_job = jobs::SyncToGitIndex::new(&krate.name);
            let sparse_index_job = jobs::SyncToSparseIndex::new(&krate.name);
            let delete_from_storage_job = jobs::DeleteCrateFromStorage::new(name);

            tokio::try_join!(
                git_index_job.enqueue(conn),
                sparse_index_job.enqueue(conn),
                delete_from_storage_job.enqueue(conn),
            )?;

            Ok::<_, BoxedAppError>(())
        }
        .scope_boxed()
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn find_crate(conn: &mut AsyncPgConnection, name: &str) -> QueryResult<Option<Crate>> {
    Crate::by_name(name).first(conn).await.optional()
}

async fn get_crate_downloads(conn: &mut AsyncPgConnection, crate_id: i32) -> QueryResult<u64> {
    let downloads = crate_downloads::table
        .find(crate_id)
        .select(crate_downloads::downloads)
        .first::<i64>(conn)
        .await
        .optional()?;

    Ok(downloads.unwrap_or_default().to_u64().unwrap_or(u64::MAX))
}

fn max_downloads(age: &TimeDelta) -> u64 {
    let age_days = age.num_days().to_u64().unwrap_or(u64::MAX);
    let age_months = age_days.div_ceil(30);
    DOWNLOADS_PER_MONTH_LIMIT * age_months
}

async fn has_rev_dep(conn: &mut AsyncPgConnection, crate_id: i32) -> QueryResult<bool> {
    let rev_dep = dependencies::table
        .filter(dependencies::crate_id.eq(crate_id))
        .select(dependencies::id)
        .first::<i32>(conn)
        .await
        .optional()?;

    Ok(rev_dep.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::OwnerKind;
    use crate::tests::builders::{DependencyBuilder, PublishBuilder};
    use crate::tests::util::{RequestHelper, Response, TestApp};
    use crates_io_database::schema::crate_owners;
    use diesel_async::AsyncPgConnection;
    use http::StatusCode;
    use insta::assert_snapshot;
    use serde_json::json;

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
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(response.body().is_empty());

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
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(response.body().is_empty());

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
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(response.body().is_empty());

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
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);

        assert_crate_exists(&anon, "foo", true).await;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_token_auth() -> anyhow::Result<()> {
        let (_app, anon, user, token) = TestApp::full().with_token().await;

        publish_crate(&user, "foo").await;

        let response = delete_crate(&token, "foo").await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action can only be performed on the crates.io website"}]}"#);

        assert_crate_exists(&anon, "foo", true).await;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_missing_crate() -> anyhow::Result<()> {
        let (_app, _anon, user) = TestApp::full().with_user().await;

        let response = delete_crate(&user, "foo").await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate `foo` does not exist"}]}"#);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_not_owner() -> anyhow::Result<()> {
        let (app, anon, user) = TestApp::full().with_user().await;
        let user2 = app.db_new_user("bar").await;

        publish_crate(&user, "foo").await;

        let response = delete_crate(&user2, "foo").await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
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
        assert_eq!(response.status(), StatusCode::OK);

        let response = delete_crate(&user2, "foo").await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
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
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
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
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"only crates with less than 100 downloads per month can be deleted after 72 hours"}]}"#);

        assert_crate_exists(&anon, "foo", true).await;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_rev_deps() -> anyhow::Result<()> {
        let (app, anon, user) = TestApp::full().with_user().await;
        let mut conn = app.db_conn().await;

        publish_crate(&user, "foo").await;
        adjust_creation_date(&mut conn, "foo", 73).await?;

        // Publish another crate
        let pb = PublishBuilder::new("bar", "1.0.0").dependency(DependencyBuilder::new("foo"));
        let response = user.publish_crate(pb).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = delete_crate(&user, "foo").await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"only crates without reverse dependencies can be deleted after 72 hours"}]}"#);

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
}
