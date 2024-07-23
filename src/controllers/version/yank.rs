//! Endpoints for yanking and unyanking specific versions of crates

use super::version_and_crate;
use crate::auth::AuthCheck;
use crate::controllers::cargo_prelude::*;
use crate::models::token::EndpointScope;
use crate::models::Rights;
use crate::models::{insert_version_owner_action, VersionAction};
use crate::rate_limiter::LimitedAction;
use crate::schema::versions;
use crate::util::errors::{custom, version_not_found};
use crate::worker::jobs;
use crate::worker::jobs::UpdateDefaultVersion;
use crates_io_worker::BackgroundJob;
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use tokio::runtime::Handle;

/// Handles the `DELETE /crates/:crate_id/:version/yank` route.
/// This does not delete a crate version, it makes the crate
/// version accessible only to crates that already have a
/// `Cargo.lock` containing this version.
///
/// Notes:
/// Crate deletion is not implemented to avoid breaking builds,
/// and the goal of yanking a crate is to prevent crates
/// beginning to depend on the yanked crate version.
pub async fn yank(
    app: AppState,
    Path((crate_name, version)): Path<(String, String)>,
    req: Parts,
) -> AppResult<Response> {
    modify_yank(crate_name, version, app, req, true).await
}

/// Handles the `PUT /crates/:crate_id/:version/unyank` route.
pub async fn unyank(
    app: AppState,
    Path((crate_name, version)): Path<(String, String)>,
    req: Parts,
) -> AppResult<Response> {
    modify_yank(crate_name, version, app, req, false).await
}

/// Changes `yanked` flag on a crate version record
async fn modify_yank(
    crate_name: String,
    version: String,
    state: AppState,
    req: Parts,
    yanked: bool,
) -> AppResult<Response> {
    // FIXME: Should reject bad requests before authentication, but can't due to
    // lifetime issues with `req`.

    if semver::Version::parse(&version).is_err() {
        return Err(version_not_found(&crate_name, &version));
    }

    let conn = state.db_write().await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        let auth = AuthCheck::default()
            .with_endpoint_scope(EndpointScope::Yank)
            .for_crate(&crate_name)
            .check(&req, conn)?;

        state
            .rate_limiter
            .check_rate_limit(auth.user_id(), LimitedAction::YankUnyank, conn)?;

        let (version, krate) = version_and_crate(conn, &crate_name, &version)?;
        let api_token_id = auth.api_token_id();
        let user = auth.user();
        let owners = krate.owners(conn)?;

        if Handle::current().block_on(user.rights(&state, &owners))? < Rights::Publish {
            if user.is_admin {
                let action = if yanked { "yanking" } else { "unyanking" };
                warn!(
                    "Admin {} is {action} {}@{}",
                    user.gh_login, krate.name, version.num
                );
            } else {
                return Err(custom(
                    StatusCode::FORBIDDEN,
                    "must already be an owner to yank or unyank",
                ));
            }
        }

        if version.yanked == yanked {
            // The crate is already in the state requested, nothing to do
            return ok_true();
        }

        diesel::update(&version)
            .set(versions::yanked.eq(yanked))
            .execute(conn)?;

        let action = if yanked {
            VersionAction::Yank
        } else {
            VersionAction::Unyank
        };

        insert_version_owner_action(conn, version.id, user.id, api_token_id, action)?;

        jobs::enqueue_sync_to_index(&krate.name, conn)?;

        UpdateDefaultVersion::new(krate.id).enqueue(conn)?;

        ok_true()
    })
    .await
}
