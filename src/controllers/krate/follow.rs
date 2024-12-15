//! Endpoints for managing a per user list of followed crates

use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::controllers::helpers::ok_true;
use crate::controllers::krate::CratePath;
use crate::models::{Crate, Follow};
use crate::schema::*;
use crate::util::errors::{crate_not_found, AppResult};
use axum::response::Response;
use axum_extra::json;
use axum_extra::response::ErasedJson;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use http::request::Parts;

async fn follow_target(
    crate_name: &str,
    conn: &mut AsyncPgConnection,
    user_id: i32,
) -> AppResult<Follow> {
    let crate_id = Crate::by_name(crate_name)
        .select(crates::id)
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| crate_not_found(crate_name))?;

    Ok(Follow { user_id, crate_id })
}

/// Follow a crate.
#[utoipa::path(
    put,
    path = "/api/v1/crates/{name}/follow",
    params(CratePath),
    tag = "crates",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn follow_crate(app: AppState, path: CratePath, req: Parts) -> AppResult<Response> {
    let mut conn = app.db_write().await?;
    let user_id = AuthCheck::default().check(&req, &mut conn).await?.user_id();
    let follow = follow_target(&path.name, &mut conn, user_id).await?;
    diesel::insert_into(follows::table)
        .values(&follow)
        .on_conflict_do_nothing()
        .execute(&mut conn)
        .await?;

    ok_true()
}

/// Unfollow a crate.
#[utoipa::path(
    delete,
    path = "/api/v1/crates/{name}/follow",
    params(CratePath),
    tag = "crates",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn unfollow_crate(app: AppState, path: CratePath, req: Parts) -> AppResult<Response> {
    let mut conn = app.db_write().await?;
    let user_id = AuthCheck::default().check(&req, &mut conn).await?.user_id();
    let follow = follow_target(&path.name, &mut conn, user_id).await?;
    diesel::delete(&follow).execute(&mut conn).await?;

    ok_true()
}

/// Check if a crate is followed.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/following",
    params(CratePath),
    tag = "crates",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn get_following_crate(
    app: AppState,
    path: CratePath,
    req: Parts,
) -> AppResult<ErasedJson> {
    use diesel::dsl::exists;

    let mut conn = app.db_read_prefer_primary().await?;
    let user_id = AuthCheck::only_cookie()
        .check(&req, &mut conn)
        .await?
        .user_id();

    let follow = follow_target(&path.name, &mut conn, user_id).await?;
    let following = diesel::select(exists(follows::table.find(follow.id())))
        .get_result::<bool>(&mut conn)
        .await?;

    Ok(json!({ "following": following }))
}
