//! Endpoints for managing a per user list of followed crates

use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::controllers::helpers::ok_true;
use crate::models::{Crate, Follow};
use crate::schema::*;
use crate::tasks::spawn_blocking;
use crate::util::diesel::Conn;
use crate::util::errors::{crate_not_found, AppResult};
use axum::extract::Path;
use axum::response::Response;
use axum::Json;
use diesel::prelude::*;
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use http::request::Parts;
use serde_json::Value;

fn follow_target(crate_name: &str, conn: &mut impl Conn, user_id: i32) -> AppResult<Follow> {
    let crate_id = Crate::by_name(crate_name)
        .select(crates::id)
        .first(conn)
        .optional()?
        .ok_or_else(|| crate_not_found(crate_name))?;

    Ok(Follow { user_id, crate_id })
}

/// Handles the `PUT /crates/:crate_id/follow` route.
pub async fn follow(
    app: AppState,
    Path(crate_name): Path<String>,
    req: Parts,
) -> AppResult<Response> {
    let conn = app.db_write().await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        let user_id = AuthCheck::default().check(&req, conn)?.user_id();
        let follow = follow_target(&crate_name, conn, user_id)?;
        diesel::insert_into(follows::table)
            .values(&follow)
            .on_conflict_do_nothing()
            .execute(conn)?;

        ok_true()
    })
    .await
}

/// Handles the `DELETE /crates/:crate_id/follow` route.
pub async fn unfollow(
    app: AppState,
    Path(crate_name): Path<String>,
    req: Parts,
) -> AppResult<Response> {
    let conn = app.db_write().await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        let user_id = AuthCheck::default().check(&req, conn)?.user_id();
        let follow = follow_target(&crate_name, conn, user_id)?;
        diesel::delete(&follow).execute(conn)?;

        ok_true()
    })
    .await
}

/// Handles the `GET /crates/:crate_id/following` route.
pub async fn following(
    app: AppState,
    Path(crate_name): Path<String>,
    req: Parts,
) -> AppResult<Json<Value>> {
    let conn = app.db_read_prefer_primary().await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        use diesel::dsl::exists;

        let user_id = AuthCheck::only_cookie().check(&req, conn)?.user_id();
        let follow = follow_target(&crate_name, conn, user_id)?;
        let following =
            diesel::select(exists(follows::table.find(follow.id()))).get_result::<bool>(conn)?;

        Ok(Json(json!({ "following": following })))
    })
    .await
}
