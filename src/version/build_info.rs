use conduit::{Request, Response};
use serde_json;

use app::RequestApp;
use db::RequestTransaction;
use models::{Rights, Version};
use owner::rights;
use user::RequestUser;
use util::{human, CargoResult, RequestUtils};
use version::version_and_crate;
use views::EncodableVersionBuildInfoUpload;

use schema::*;

#[derive(Clone, Identifiable, Associations, Debug, Queryable)]
#[belongs_to(Version)]
#[table_name = "build_info"]
#[primary_key(version_id, rust_version, target)]
/// Stores information about whether this version built on the specified Rust version and target.
pub struct BuildInfo {
    version_id: i32,
    pub rust_version: String,
    pub target: String,
    pub passed: bool,
}

/// Handles the `POST /crates/:crate_id/:version/build_info` route for the
/// `cargo publish-build-info` command to report on which versions of Rust
/// a crate builds with.
pub fn publish_build_info(req: &mut Request) -> CargoResult<Response> {
    let mut body = String::new();
    req.body().read_to_string(&mut body)?;
    let info: EncodableVersionBuildInfoUpload = serde_json::from_str(&body)
        .map_err(|e| human(&format_args!("invalid upload request: {}", e)))?;

    let (version, krate) = version_and_crate(req)?;
    let user = req.user()?;
    let tx = req.db_conn()?;
    let owners = krate.owners(&tx)?;
    if rights(req.app(), &owners, user)? < Rights::Publish {
        return Err(human("must already be an owner to publish build info"));
    }

    version.store_build_info(&tx, info)?;

    #[derive(Serialize)]
    struct R {
        ok: bool,
    }
    Ok(req.json(&R { ok: true }))
}
