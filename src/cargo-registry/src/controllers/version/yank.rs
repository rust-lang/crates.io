//! Endpoints for yanking and unyanking specific versions of crates

use swirl::Job;

use super::version_and_crate;
use crate::controllers::cargo_prelude::*;
use crate::git;
use crate::models::Rights;
use crate::models::{insert_version_owner_action, VersionAction};

/// Handles the `DELETE /crates/:crate_id/:version/yank` route.
/// This does not delete a crate version, it makes the crate
/// version accessible only to crates that already have a
/// `Cargo.lock` containing this version.
///
/// Notes:
/// Crate deletion is not implemented to avoid breaking builds,
/// and the goal of yanking a crate is to prevent crates
/// beginning to depend on the yanked crate version.
pub fn yank(req: &mut dyn RequestExt) -> EndpointResult {
    modify_yank(req, true)
}

/// Handles the `PUT /crates/:crate_id/:version/unyank` route.
pub fn unyank(req: &mut dyn RequestExt) -> EndpointResult {
    modify_yank(req, false)
}

/// Changes `yanked` flag on a crate version record
fn modify_yank(req: &mut dyn RequestExt, yanked: bool) -> EndpointResult {
    let authenticated_user = req.authenticate()?;
    let (conn, version, krate) = version_and_crate(req)?;
    let api_token_id = authenticated_user.api_token_id();
    let user = authenticated_user.user();
    let owners = krate.owners(&conn)?;

    if user.rights(req.app(), &owners)? < Rights::Publish {
        return Err(cargo_err("must already be an owner to yank or unyank"));
    }
    let action = if yanked {
        VersionAction::Yank
    } else {
        VersionAction::Unyank
    };

    insert_version_owner_action(&conn, version.id, user.id, api_token_id, action)?;

    git::yank(krate.name, version, yanked).enqueue(&conn)?;

    ok_true()
}
