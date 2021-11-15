//! Endpoints for yanking and unyanking specific versions of crates

use swirl::Job;

use super::{extract_crate_name_and_semver, version_and_crate};
use crate::controllers::cargo_prelude::*;
use crate::models::Rights;
use crate::models::{insert_version_owner_action, VersionAction};
use crate::schema::versions;
use crate::worker;

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
    // FIXME: Should reject bad requests before authentication, but can't due to
    // lifetime issues with `req`.
    let authenticated_user = req.authenticate()?;
    let (crate_name, semver) = extract_crate_name_and_semver(req)?;

    let conn = req.db_conn()?;
    let (version, krate) = version_and_crate(&conn, crate_name, semver)?;
    let api_token_id = authenticated_user.api_token_id();
    let user = authenticated_user.user();
    let owners = krate.owners(&conn)?;

    if user.rights(req.app(), &owners)? < Rights::Publish {
        return Err(cargo_err("must already be an owner to yank or unyank"));
    }

    if version.yanked == yanked {
        // The crate is already in the state requested, nothing to do
        return ok_true();
    }

    diesel::update(&version)
        .set(versions::yanked.eq(yanked))
        .execute(&*conn)?;

    let action = if yanked {
        VersionAction::Yank
    } else {
        VersionAction::Unyank
    };

    insert_version_owner_action(&conn, version.id, user.id, api_token_id, action)?;

    worker::sync_yanked(krate.name, version.num).enqueue(&conn)?;

    ok_true()
}
