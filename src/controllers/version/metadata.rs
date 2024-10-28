//! Endpoints that expose metadata about crate versions
//!
//! These endpoints provide data that could be obtained directly from the
//! index or cached metadata which was extracted (client side) from the
//! `Cargo.toml` file.

use axum::extract::Path;
use axum::Json;
use crates_io_database::schema::{crates, dependencies};
use crates_io_worker::BackgroundJob;
use diesel::{
    BelongingToDsl, BoolExpressionMethods, ExpressionMethods, PgExpressionMethods, QueryDsl,
    RunQueryDsl, SelectableHelper,
};
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use http::request::Parts;
use http::StatusCode;
use serde::Deserialize;
use serde_json::Value;
use tokio::runtime::Handle;

use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::models::token::EndpointScope;
use crate::models::{
    insert_version_owner_action, Crate, Dependency, Rights, Version, VersionAction,
    VersionOwnerAction,
};
use crate::rate_limiter::LimitedAction;
use crate::schema::versions;
use crate::tasks::spawn_blocking;
use crate::util::diesel::Conn;
use crate::util::errors::{bad_request, custom, version_not_found, AppResult};
use crate::views::{EncodableDependency, EncodableVersion};
use crate::worker::jobs::{SyncToGitIndex, SyncToSparseIndex, UpdateDefaultVersion};

use super::version_and_crate;

#[derive(Deserialize)]
pub struct VersionUpdate {
    yanked: Option<bool>,
    yank_message: Option<String>,
}
#[derive(Deserialize)]
pub struct VersionUpdateRequest {
    version: VersionUpdate,
}

/// Handles the `GET /crates/:crate_id/:version/dependencies` route.
///
/// This information can be obtained directly from the index.
///
/// In addition to returning cached data from the index, this returns
/// fields for `id`, `version_id`, and `downloads` (which appears to always
/// be 0)
pub async fn dependencies(
    state: AppState,
    Path((crate_name, version)): Path<(String, String)>,
) -> AppResult<Json<Value>> {
    if semver::Version::parse(&version).is_err() {
        return Err(version_not_found(&crate_name, &version));
    }

    let mut conn = state.db_read().await?;
    let (version, _) = version_and_crate(&mut conn, &crate_name, &version).await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        let deps = Dependency::belonging_to(&version)
            .inner_join(crates::table)
            .select((Dependency::as_select(), crates::name))
            .order((dependencies::optional, crates::name))
            .load::<(Dependency, String)>(conn)?
            .into_iter()
            .map(|(dep, crate_name)| EncodableDependency::from_dep(dep, &crate_name))
            .collect::<Vec<_>>();

        Ok(Json(json!({ "dependencies": deps })))
    })
    .await
}

/// Handles the `GET /crates/:crate_id/:version/authors` route.
pub async fn authors() -> Json<Value> {
    // Currently we return the empty list.
    // Because the API is not used anymore after RFC https://github.com/rust-lang/rfcs/pull/3052.

    Json(json!({
        "users": [],
        "meta": { "names": [] },
    }))
}

/// Handles the `GET /crates/:crate/:version` route.
///
/// The frontend doesn't appear to hit this endpoint, but our tests do, and it seems to be a useful
/// API route to have.
pub async fn show(
    state: AppState,
    Path((crate_name, version)): Path<(String, String)>,
) -> AppResult<Json<Value>> {
    if semver::Version::parse(&version).is_err() {
        return Err(version_not_found(&crate_name, &version));
    }

    let mut conn = state.db_read().await?;
    let (version, krate) = version_and_crate(&mut conn, &crate_name, &version).await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        let published_by = version.published_by(conn);
        let actions = VersionOwnerAction::by_version(conn, &version)?;

        let version = EncodableVersion::from(version, &krate.name, published_by, actions);
        Ok(Json(json!({ "version": version })))
    })
    .await
}

/// Handles the `PATCH /crates/:crate/:version` route.
///
/// This endpoint allows updating the yanked state of a version, including a yank message.
pub async fn update(
    state: AppState,
    Path((crate_name, version)): Path<(String, String)>,
    req: Parts,
    Json(update_request): Json<VersionUpdateRequest>,
) -> AppResult<Json<Value>> {
    if semver::Version::parse(&version).is_err() {
        return Err(version_not_found(&crate_name, &version));
    }

    let mut conn = state.db_write().await?;
    let (mut version, krate) = version_and_crate(&mut conn, &crate_name, &version).await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        validate_yank_update(&update_request.version, &version)?;
        perform_version_yank_update(
            &state,
            &req,
            conn,
            &mut version,
            &krate,
            update_request.version.yanked,
            update_request.version.yank_message,
        )?;

        let published_by = version.published_by(conn);
        let actions = VersionOwnerAction::by_version(conn, &version)?;
        let updated_version = EncodableVersion::from(version, &krate.name, published_by, actions);
        Ok(Json(json!({ "version": updated_version })))
    })
    .await
}

fn validate_yank_update(update_data: &VersionUpdate, version: &Version) -> AppResult<()> {
    if update_data.yank_message.is_some() {
        if matches!(update_data.yanked, Some(false)) {
            return Err(bad_request("Cannot set yank message when unyanking"));
        }

        if update_data.yanked.is_none() && !version.yanked {
            return Err(bad_request(
                "Cannot update yank message for a version that is not yanked",
            ));
        }
    }

    Ok(())
}

pub fn perform_version_yank_update(
    state: &AppState,
    req: &Parts,
    conn: &mut impl Conn,
    version: &mut Version,
    krate: &Crate,
    yanked: Option<bool>,
    yank_message: Option<String>,
) -> AppResult<()> {
    let auth = AuthCheck::default()
        .with_endpoint_scope(EndpointScope::Yank)
        .for_crate(&krate.name)
        .check(req, conn)?;

    state
        .rate_limiter
        .check_rate_limit(auth.user_id(), LimitedAction::YankUnyank, conn)?;

    let api_token_id = auth.api_token_id();
    let user = auth.user();
    let owners = krate.owners(conn)?;

    let yanked = yanked.unwrap_or(version.yanked);

    if Handle::current().block_on(user.rights(state, &owners))? < Rights::Publish {
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

    // Check if the yanked state or yank message has changed and update if necessary
    let updated_cnt = diesel::update(
        versions::table.find(version.id).filter(
            versions::yanked
                .is_distinct_from(yanked)
                .or(versions::yank_message.is_distinct_from(&yank_message)),
        ),
    )
    .set((
        versions::yanked.eq(yanked),
        versions::yank_message.eq(&yank_message),
    ))
    .execute(conn)?;

    // If no rows were updated, return early
    if updated_cnt == 0 {
        return Ok(());
    }

    // Apply the update to the version
    version.yanked = yanked;
    version.yank_message = yank_message;

    let action = if yanked {
        VersionAction::Yank
    } else {
        VersionAction::Unyank
    };
    insert_version_owner_action(conn, version.id, user.id, api_token_id, action)?;

    SyncToGitIndex::new(&krate.name).enqueue(conn)?;
    SyncToSparseIndex::new(&krate.name).enqueue(conn)?;
    UpdateDefaultVersion::new(krate.id).enqueue(conn)?;

    Ok(())
}
