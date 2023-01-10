//! Endpoints for managing a per user list of followed crates

use crate::auth::AuthCheck;
use diesel::associations::Identifiable;

use crate::controllers::frontend_prelude::*;
use crate::db::DieselPooledConn;
use crate::models::{Crate, Follow};
use crate::schema::*;

fn follow_target(crate_name: &str, conn: &DieselPooledConn<'_>, user_id: i32) -> AppResult<Follow> {
    let crate_id = Crate::by_name(crate_name)
        .select(crates::id)
        .first(&**conn)?;
    Ok(Follow { user_id, crate_id })
}

/// Handles the `PUT /crates/:crate_id/follow` route.
pub async fn follow(Path(crate_name): Path<String>, req: Parts) -> AppResult<Response> {
    conduit_compat(move || {
        let conn = req.app().db_write()?;
        let user_id = AuthCheck::default().check(&req, &conn)?.user_id();
        let follow = follow_target(&crate_name, &conn, user_id)?;
        diesel::insert_into(follows::table)
            .values(&follow)
            .on_conflict_do_nothing()
            .execute(&*conn)?;

        ok_true()
    })
    .await
}

/// Handles the `DELETE /crates/:crate_id/follow` route.
pub async fn unfollow(Path(crate_name): Path<String>, req: Parts) -> AppResult<Response> {
    conduit_compat(move || {
        let conn = req.app().db_write()?;
        let user_id = AuthCheck::default().check(&req, &conn)?.user_id();
        let follow = follow_target(&crate_name, &conn, user_id)?;
        diesel::delete(&follow).execute(&*conn)?;

        ok_true()
    })
    .await
}

/// Handles the `GET /crates/:crate_id/following` route.
pub async fn following(Path(crate_name): Path<String>, req: Parts) -> AppResult<Json<Value>> {
    conduit_compat(move || {
        use diesel::dsl::exists;

        let conn = req.app().db_read_prefer_primary()?;
        let user_id = AuthCheck::only_cookie().check(&req, &conn)?.user_id();
        let follow = follow_target(&crate_name, &conn, user_id)?;
        let following =
            diesel::select(exists(follows::table.find(follow.id()))).get_result::<bool>(&*conn)?;

        Ok(Json(json!({ "following": following })))
    })
    .await
}
