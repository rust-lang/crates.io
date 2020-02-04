//! Endpoints for managing a per user list of followed crates

use diesel::associations::Identifiable;

use crate::controllers::frontend_prelude::*;
use crate::db::DieselPooledConn;
use crate::models::{Crate, Follow};
use crate::schema::*;

fn follow_target(req: &dyn Request, conn: &DieselPooledConn<'_>) -> AppResult<Follow> {
    let user_id = req.authenticate(conn)?.user_id();
    let crate_name = &req.params()["crate_id"];
    let crate_id = Crate::by_name(crate_name)
        .select(crates::id)
        .first(&**conn)?;
    Ok(Follow { user_id, crate_id })
}

/// Handles the `PUT /crates/:crate_id/follow` route.
pub fn follow(req: &mut dyn Request) -> AppResult<Response> {
    let conn = req.db_conn()?;
    let follow = follow_target(req, &conn)?;
    diesel::insert_into(follows::table)
        .values(&follow)
        .on_conflict_do_nothing()
        .execute(&*conn)?;

    ok_true()
}

/// Handles the `DELETE /crates/:crate_id/follow` route.
pub fn unfollow(req: &mut dyn Request) -> AppResult<Response> {
    let conn = req.db_conn()?;
    let follow = follow_target(req, &conn)?;
    diesel::delete(&follow).execute(&*conn)?;

    ok_true()
}

/// Handles the `GET /crates/:crate_id/following` route.
pub fn following(req: &mut dyn Request) -> AppResult<Response> {
    use diesel::dsl::exists;

    let conn = req.db_conn()?;
    let follow = follow_target(req, &conn)?;
    let following = diesel::select(exists(follows::table.find(follow.id()))).get_result(&*conn)?;

    #[derive(Serialize)]
    struct R {
        following: bool,
    }
    Ok(req.json(&R { following }))
}
