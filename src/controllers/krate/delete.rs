use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::controllers::helpers::authorization::Rights;
use crate::controllers::krate::CratePath;
use crate::email::EmailMessage;
use crate::models::NewDeletedCrate;
use crate::schema::{crate_downloads, crates, dependencies};
use crate::util::errors::{AppResult, BoxedAppError, custom};
use crate::worker::jobs;
use axum::extract::rejection::QueryRejection;
use axum::extract::{FromRequestParts, Query};
use bigdecimal::ToPrimitive;
use chrono::{TimeDelta, Utc};
use crates_io_database::models::GitIndexSyncQueueItem;
use crates_io_database::schema::deleted_crates;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use http::StatusCode;
use http::request::Parts;
use minijinja::context;
use serde::Deserialize;
use tracing::error;

pub const DOWNLOADS_PER_MONTH_LIMIT: u64 = 1000;
const AVAILABLE_AFTER: TimeDelta = TimeDelta::hours(24);

#[derive(Debug, Deserialize, FromRequestParts, utoipa::IntoParams)]
#[from_request(via(Query), rejection(QueryRejection))]
#[into_params(parameter_in = Query)]
pub struct DeleteQueryParams {
    message: Option<String>,
}

impl DeleteQueryParams {
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref().filter(|m| !m.is_empty())
    }
}

/// Delete a crate.
///
/// The crate is immediately deleted from the database, and with a small delay
/// from the git and sparse index, and the crate file storage.
///
/// The crate can only be deleted by the owner of the crate, and only if the
/// crate has been published for less than 72 hours, or if the crate has a
/// single owner, has been downloaded less than 1000 times for each month it has
/// been published, and is not depended upon by any other crate on crates.io.
#[utoipa::path(
    delete,
    path = "/api/v1/crates/{name}",
    params(CratePath, DeleteQueryParams),
    security(("cookie" = [])),
    tag = "crates",
    responses((status = 204, description = "Successful Response")),
)]
pub async fn delete_crate(
    path: CratePath,
    params: DeleteQueryParams,
    parts: Parts,
    app: AppState,
) -> AppResult<StatusCode> {
    let mut conn = app.db_write().await?;

    // Check that the user is authenticated
    let auth = AuthCheck::only_cookie().check(&parts, &mut conn).await?;

    // Check that the crate exists
    let krate = path.load_crate(&mut conn).await?;

    // Check that the user is an owner of the crate (team owners are not allowed to delete crates)
    let user = auth.user();
    let owners = krate.owners(&mut conn).await?;
    match Rights::get(user, &*app.github, &owners, &app.config.gh_token_encryption).await? {
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

    let created_at = krate.created_at;

    let age = Utc::now().signed_duration_since(created_at);
    if age > TimeDelta::hours(72) {
        if owners.len() > 1 {
            let msg = "only crates with a single owner can be deleted after 72 hours";
            return Err(custom(StatusCode::UNPROCESSABLE_ENTITY, msg));
        }

        let downloads = get_crate_downloads(&mut conn, krate.id).await?;
        if downloads > max_downloads(&age) {
            let msg = format!(
                "only crates with less than {DOWNLOADS_PER_MONTH_LIMIT} downloads per month can be deleted after 72 hours"
            );
            return Err(custom(StatusCode::UNPROCESSABLE_ENTITY, msg));
        }
    }

    // All crates with reverse dependencies are blocked from being deleted to avoid unexpected
    // historical index changes.
    if has_rev_dep(&mut conn, krate.id).await? {
        let msg = "only crates without reverse dependencies can be deleted";
        return Err(custom(StatusCode::UNPROCESSABLE_ENTITY, msg));
    }

    let crate_name = krate.name.clone();
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
                .maybe_message(params.message())
                .build();

            diesel::insert_into(deleted_crates::table)
                .values(deleted_crate)
                .execute(conn)
                .await?;

            GitIndexSyncQueueItem::queue(conn, &krate.name).await?;
            let sparse_index_job = jobs::SyncToSparseIndex::new(&krate.name);
            let delete_from_storage_job = jobs::DeleteCrateFromStorage::new(path.name);

            tokio::try_join!(
                jobs::SyncToGitIndex.enqueue(conn),
                sparse_index_job.enqueue(conn),
                delete_from_storage_job.enqueue(conn),
            )?;

            Ok::<_, BoxedAppError>(())
        }
        .scope_boxed()
    })
    .await?;

    let email_future = async {
        if let Some(recipient) = user.email(&mut conn).await? {
            let email = EmailMessage::from_template(
                "crate_deletion",
                context! {
                    user => user.gh_login,
                    krate => crate_name
                },
            )?;

            app.emails.send(&recipient, email).await?
        }

        Ok::<_, anyhow::Error>(())
    };

    if let Err(err) = email_future.await {
        error!("Failed to send crate deletion email: {err}");
    }

    Ok(StatusCode::NO_CONTENT)
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

pub fn max_downloads(age: &TimeDelta) -> u64 {
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
