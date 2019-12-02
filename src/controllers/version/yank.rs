//! Endpoints for yanking and unyanking specific versions of crates

use swirl::Job;

use super::version_and_crate;
use crate::controllers::prelude::*;
use crate::git;
use crate::models::{insert_version_owner_action, Rights, VersionAction};
use crate::util::AppError;

/// Handles the `DELETE /crates/:crate_id/:version/yank` route.
/// This does not delete a crate version, it makes the crate
/// version accessible only to crates that already have a
/// `Cargo.lock` containing this version.
///
/// Notes:
/// Crate deletion is not implemented to avoid breaking builds,
/// and the goal of yanking a crate is to prevent crates
/// beginning to depend on the yanked crate version.
pub fn yank(req: &mut dyn Request) -> AppResult<Response> {
    modify_yank(req, true)
}

/// Handles the `PUT /crates/:crate_id/:version/unyank` route.
pub fn unyank(req: &mut dyn Request) -> AppResult<Response> {
    modify_yank(req, false)
}

/// Changes `yanked` flag on a crate version record
fn modify_yank(req: &mut dyn Request, yanked: bool) -> AppResult<Response> {
    let (version, krate) = version_and_crate(req)?;
    let user = req.user()?;
    let conn = req.db_conn()?;
    let owners = krate.owners(&conn)?;
    if user.rights(req.app(), &owners)? < Rights::Publish {
        return Err(cargo_err("must already be an owner to yank or unyank"));
    }
    let action = if yanked {
        VersionAction::Yank
    } else {
        VersionAction::Unyank
    };
    let api_token_id = req.authentication_source()?.api_token_id();

    insert_version_owner_action(&conn, version.id, user.id, api_token_id, action)?;

    git::yank(krate.name, version, yanked)
        .enqueue(&conn)
        .map_err(|e| AppError::from_std_error(e))?;

    ok_true()
}
