//! Endpoints for managing a per user list of followed crates

use crate::auth::AuthCheck;
use diesel::associations::Identifiable;

use crate::controllers::frontend_prelude::*;
use crate::db::DieselPooledConn;
use crate::models::{Crate, Follow};
use crate::schema::*;

fn follow_target(
    req: &ConduitRequest,
    conn: &DieselPooledConn<'_>,
    user_id: i32,
) -> AppResult<Follow> {
    let crate_name = req.param("crate_id").unwrap();
    let crate_id = Crate::by_name(crate_name)
        .select(crates::id)
        .first(&**conn)?;
    Ok(Follow { user_id, crate_id })
}

/// Handles the `PUT /crates/:crate_id/follow` route.
pub fn follow(req: ConduitRequest) -> AppResult<Response> {
    let user_id = AuthCheck::default().check(&req)?.user_id();
    let conn = req.app().db_write()?;
    let follow = follow_target(&req, &conn, user_id)?;
    diesel::insert_into(follows::table)
        .values(&follow)
        .on_conflict_do_nothing()
        .execute(&*conn)?;

    ok_true()
}

/// Handles the `DELETE /crates/:crate_id/follow` route.
pub fn unfollow(req: ConduitRequest) -> AppResult<Response> {
    let user_id = AuthCheck::default().check(&req)?.user_id();
    let conn = req.app().db_write()?;
    let follow = follow_target(&req, &conn, user_id)?;
    diesel::delete(&follow).execute(&*conn)?;

    ok_true()
}

/// Handles the `GET /crates/:crate_id/following` route.
pub fn following(req: ConduitRequest) -> AppResult<Json<Value>> {
    use diesel::dsl::exists;

    let user_id = AuthCheck::only_cookie().check(&req)?.user_id();
    let conn = req.app().db_read_prefer_primary()?;
    let follow = follow_target(&req, &conn, user_id)?;
    let following =
        diesel::select(exists(follows::table.find(follow.id()))).get_result::<bool>(&*conn)?;

    Ok(Json(json!({ "following": following })))
}
