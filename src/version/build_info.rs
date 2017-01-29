use std::str::FromStr;

use chrono::{DateTime, NaiveDate, Utc};
use conduit::{Request, Response};
use semver;
use serde_json;

use app::RequestApp;
use db::RequestTransaction;
use models::{Rights, Version};
use owner::rights;
use user::RequestUser;
use util::{human, CargoResult, RequestUtils};
use version::version_and_crate;
use views::{EncodableMaxVersionBuildInfo, EncodableVersionBuildInfoUpload,
            ParsedRustChannelVersion};

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

/// The columns to select from the `build_info` table. The table also stores `created_at` and
/// `updated_at` metadata for each row, but we're not displaying those anywhere so we're not
/// bothering to select them.
pub const BUILD_INFO_FIELDS: (
    build_info::version_id,
    build_info::rust_version,
    build_info::target,
    build_info::passed,
) = (
    build_info::version_id,
    build_info::rust_version,
    build_info::target,
    build_info::passed,
);

#[derive(Debug)]
/// The maximum version of Rust from each channel that a crate version successfully builds with.
/// Used for summarizing this information in badge form on crate list pages.
pub struct MaxBuildInfo {
    pub stable: Option<semver::Version>,
    pub beta: Option<NaiveDate>,
    pub nightly: Option<NaiveDate>,
}

impl MaxBuildInfo {
    /// Encode stable semver number as a string and beta and nightly as times appropriate for
    /// JSON.
    pub fn encode(self) -> EncodableMaxVersionBuildInfo {
        fn naive_date_to_rfc3339(date: NaiveDate) -> String {
            DateTime::<Utc>::from_utc(date.and_hms(0, 0, 0), Utc).to_rfc3339()
        }

        EncodableMaxVersionBuildInfo {
            stable: self.stable.map(|v| v.to_string()),
            beta: self.beta.map(naive_date_to_rfc3339),
            nightly: self.nightly.map(naive_date_to_rfc3339),
        }
    }
}

impl BuildInfo {
    /// From a set of build information data, Find the largest or latest Rust versions that we know
    /// about for each channel. Stable uses the largest semver version number; beta and nightly use
    /// the latest date.
    pub fn max<I>(build_infos: I) -> CargoResult<MaxBuildInfo>
    where
        I: IntoIterator<Item = BuildInfo>,
    {
        let build_infos = build_infos
            .into_iter()
            .map(|bi| ParsedRustChannelVersion::from_str(&bi.rust_version))
            .collect::<Result<Vec<_>, _>>()?;

        let stable = build_infos
            .iter()
            .filter_map(ParsedRustChannelVersion::as_stable)
            .max();
        let beta = build_infos
            .iter()
            .filter_map(ParsedRustChannelVersion::as_beta)
            .max();
        let nightly = build_infos
            .iter()
            .filter_map(ParsedRustChannelVersion::as_nightly)
            .max();

        Ok(MaxBuildInfo {
            stable: stable.cloned(),
            beta: beta.cloned(),
            nightly: nightly.cloned(),
        })
    }
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
