//! Endpoints for yanking and unyanking specific versions of crates

use swirl::Job;

use super::version_and_crate;
use crate::controllers::prelude::*;
use crate::git;
use crate::models::Rights;
use crate::util::CargoError;

/// Handles the `DELETE /crates/:crate_id/:version/yank` route.
/// This does not delete a crate version, it makes the crate
/// version accessible only to crates that already have a
/// `Cargo.lock` containing this version.
///
/// Notes:
/// Crate deletion is not implemented to avoid breaking builds,
/// and the goal of yanking a crate is to prevent crates
/// beginning to depend on the yanked crate version.
pub fn yank(req: &mut dyn Request) -> CargoResult<Response> {
    modify_yank(req, true)
}

/// Handles the `PUT /crates/:crate_id/:version/unyank` route.
pub fn unyank(req: &mut dyn Request) -> CargoResult<Response> {
    modify_yank(req, false)
}

/// Changes `yanked` flag on a crate version record
fn modify_yank(req: &mut dyn Request, yanked: bool) -> CargoResult<Response> {
    let (version, krate) = version_and_crate(req)?;
    let user = req.user()?;
    let conn = req.db_conn()?;
    let owners = krate.owners(&conn)?;
    if user.rights(req.app(), &owners)? < Rights::Publish {
        return Err(human("must already be an owner to yank or unyank"));
    }

    git::yank(krate.name, version, yanked)
        .enqueue(&conn)
        .map_err(|e| CargoError::from_std_error(e))?;

    #[derive(Serialize)]
    struct R {
        ok: bool,
    }
    Ok(req.json(&R { ok: true }))
}
