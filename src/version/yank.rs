use conduit::{Request, Response};
use diesel;
use diesel::prelude::*;

use app::RequestApp;
use db::RequestTransaction;
use git;
use owner::{rights, Rights};
use schema::*;
use user::RequestUser;
use util::errors::CargoError;
use util::{human, CargoResult, RequestUtils};

use super::version_and_crate;

/// Handles the `DELETE /crates/:crate_id/:version/yank` route.
/// This does not delete a crate version, it makes the crate
/// version accessible only to crates that already have a
/// `Cargo.lock` containing this version.
///
/// Notes:
/// Crate deletion is not implemented to avoid breaking builds,
/// and the goal of yanking a crate is to prevent crates
/// beginning to depend on the yanked crate version.
pub fn yank(req: &mut Request) -> CargoResult<Response> {
    modify_yank(req, true)
}

/// Handles the `PUT /crates/:crate_id/:version/unyank` route.
pub fn unyank(req: &mut Request) -> CargoResult<Response> {
    modify_yank(req, false)
}

/// Changes `yanked` flag on a crate version record
fn modify_yank(req: &mut Request, yanked: bool) -> CargoResult<Response> {
    let (version, krate) = version_and_crate(req)?;
    let user = req.user()?;
    let conn = req.db_conn()?;
    let owners = krate.owners(&conn)?;
    if rights(req.app(), &owners, user)? < Rights::Publish {
        return Err(human("must already be an owner to yank or unyank"));
    }

    if version.yanked != yanked {
        conn.transaction::<_, Box<CargoError>, _>(|| {
            diesel::update(&version)
                .set(versions::yanked.eq(yanked))
                .execute(&*conn)?;
            git::yank(&**req.app(), &krate.name, &version.num, yanked)?;
            Ok(())
        })?;
    }

    #[derive(Serialize)]
    struct R {
        ok: bool,
    }
    Ok(req.json(&R { ok: true }))
}
