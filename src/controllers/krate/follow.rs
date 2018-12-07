//! Endpoints for managing a per user list of followed crates

use diesel::associations::Identifiable;

use crate::controllers::prelude::*;
use crate::models::{Crate, Follow};
use crate::schema::*;

fn follow_target(req: &mut dyn Request) -> CargoResult<Follow> {
    let user = req.user()?;
    let conn = req.db_conn()?;
    let crate_name = &req.params()["crate_id"];
    let crate_id = Crate::by_name(crate_name)
        .select(crates::id)
        .first(&*conn)?;
    Ok(Follow {
        user_id: user.id,
        crate_id,
    })
}

/// Handles the `PUT /crates/:crate_id/follow` route.
pub fn follow(req: &mut dyn Request) -> CargoResult<Response> {
    let follow = follow_target(req)?;
    let conn = req.db_conn()?;
    diesel::insert_into(follows::table)
        .values(&follow)
        .on_conflict_do_nothing()
        .execute(&*conn)?;

    ok_true()
}

/// Handles the `DELETE /crates/:crate_id/follow` route.
pub fn unfollow(req: &mut dyn Request) -> CargoResult<Response> {
    let follow = follow_target(req)?;
    let conn = req.db_conn()?;
    diesel::delete(&follow).execute(&*conn)?;

    ok_true()
}

/// Handles the `GET /crates/:crate_id/following` route.
pub fn following(req: &mut dyn Request) -> CargoResult<Response> {
    use diesel::dsl::exists;

    let follow = follow_target(req)?;
    let conn = req.db_conn()?;
    let following = diesel::select(exists(follows::table.find(follow.id()))).get_result(&*conn)?;

    #[derive(Serialize)]
    struct R {
        following: bool,
    }
    Ok(req.json(&R { following }))
}
