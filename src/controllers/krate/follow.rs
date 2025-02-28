//! Endpoints for managing a per user list of followed crates

use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::controllers::helpers::OkResponse;
use crate::controllers::krate::CratePath;
use crate::models::{Crate, Follow};
use crate::schema::*;
use crate::util::errors::{AppResult, crate_not_found};
use axum::Json;
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
    security(
        ("api_token" = []),
        ("cookie" = []),
    ),
    tag = "crates",
    responses((status = 200, description = "Successful Response", body = inline(OkResponse))),
)]
pub async fn follow_crate(app: AppState, path: CratePath, req: Parts) -> AppResult<OkResponse> {
    let mut conn = app.db_write().await?;
    let user_id = AuthCheck::default().check(&req, &mut conn).await?.user_id();
    let follow = follow_target(&path.name, &mut conn, user_id).await?;
    diesel::insert_into(follows::table)
        .values(&follow)
        .on_conflict_do_nothing()
        .execute(&mut conn)
        .await?;

    Ok(OkResponse::new())
}

/// Unfollow a crate.
#[utoipa::path(
    delete,
    path = "/api/v1/crates/{name}/follow",
    params(CratePath),
    security(
        ("api_token" = []),
        ("cookie" = []),
    ),
    tag = "crates",
    responses((status = 200, description = "Successful Response", body = inline(OkResponse))),
)]
pub async fn unfollow_crate(app: AppState, path: CratePath, req: Parts) -> AppResult<OkResponse> {
    let mut conn = app.db_write().await?;
    let user_id = AuthCheck::default().check(&req, &mut conn).await?.user_id();
    let follow = follow_target(&path.name, &mut conn, user_id).await?;
    diesel::delete(&follow).execute(&mut conn).await?;

    Ok(OkResponse::new())
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct FollowingResponse {
    /// Whether the authenticated user is following the crate.
    pub following: bool,
}

/// Check if a crate is followed.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/following",
    params(CratePath),
    security(("cookie" = [])),
    tag = "crates",
    responses((status = 200, description = "Successful Response", body = inline(FollowingResponse))),
)]
pub async fn get_following_crate(
    app: AppState,
    path: CratePath,
    req: Parts,
) -> AppResult<Json<FollowingResponse>> {
    use diesel::dsl::exists;

    let mut conn = app.db_read_prefer_primary().await?;
    let user_id = AuthCheck::only_cookie()
        .check(&req, &mut conn)
        .await?
        .user_id();

    let follow = follow_target(&path.name, &mut conn, user_id).await?;
    let following = diesel::select(exists(follows::table.find(follow.id())))
        .get_result(&mut conn)
        .await?;

    Ok(Json(FollowingResponse { following }))
}
