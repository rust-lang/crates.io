//! Endpoints that expose metadata about crate versions
//!
//! These endpoints provide data that could be obtained directly from the
//! index or cached metadata which was extracted (client side) from the
//! `Cargo.toml` file, as well as some information stored in crates.io's
//! database.

use std::str::FromStr;

use chrono::NaiveDate;
use conduit::{Request, Response};
use diesel::prelude::*;

use db::RequestTransaction;
use util::{CargoResult, RequestUtils};

use views::{EncodableDependency, EncodablePublicUser, EncodableVersionBuildInfo,
            ParsedRustChannelVersion};
use schema::*;

use super::build_info::BuildInfo;
use super::version_and_crate;

/// Handles the `GET /crates/:crate_id/:version/dependencies` route.
///
/// This information can be obtained direclty from the index.
///
/// In addition to returning cached data from the index, this returns
/// fields for `id`, `version_id`, and `downloads` (which appears to always
/// be 0)
pub fn dependencies(req: &mut Request) -> CargoResult<Response> {
    let (version, _) = version_and_crate(req)?;
    let conn = req.db_conn()?;
    let deps = version.dependencies(&*conn)?;
    let deps = deps.into_iter()
        .map(|(dep, crate_name)| dep.encodable(&crate_name, None))
        .collect();

    #[derive(Serialize)]
    struct R {
        dependencies: Vec<EncodableDependency>,
    }
    Ok(req.json(&R { dependencies: deps }))
}

/// Handles the `GET /crates/:crate_id/:version/authors` route.
pub fn authors(req: &mut Request) -> CargoResult<Response> {
    let (version, _) = version_and_crate(req)?;
    let conn = req.db_conn()?;
    let names = version_authors::table
        .filter(version_authors::version_id.eq(version.id))
        .select(version_authors::name)
        .order(version_authors::name)
        .load(&*conn)?;

    // It was imagined that we wold associate authors with users.
    // This was never implemented. This complicated return struct
    // is all that is left, hear for backwards compatibility.
    #[derive(Serialize)]
    struct R {
        users: Vec<EncodablePublicUser>,
        meta: Meta,
    }
    #[derive(Serialize)]
    struct Meta {
        names: Vec<String>,
    }
    Ok(req.json(&R {
        users: vec![],
        meta: Meta { names: names },
    }))
}

/// Handles the `GET /crates/:crate_id/:version/build_info` route.
// We do not wish the frontend to understand how to sort Rust versions
// (semver- *or* date-based), so we return two related pieces of
// information: the ordering of all the releases in each channel and
// the pass/fail for each platform for each (channel, version) pair.
//
// {
//   "build_info": {
//     "id": 1,
//     "ordering": {
//       "nightly": ["2017-07-26"],
//       "beta": ["2017-07-18"],
//       "stable": ["1.19.0"]
//     },
//     "stable": {
//       "1.19.0": { "x86_64-apple-darwin": false }
//       "1.17.0": { "x86_64-unknown-linux-gnu": true }
//       "1.18.0": { "x86_64-pc-windows-gnu": false }
//     },
//     "beta": {
//       "2017-07-18": { "x86_64-apple-darwin": false }
//     },
//     "nightly": {
//       "2017-07-26": { "x86_64-apple-darwin": true }
//     }
//   }
// }
pub fn build_info(req: &mut Request) -> CargoResult<Response> {
    use std::collections::{BTreeSet, HashMap};

    let (version, _) = version_and_crate(req)?;

    let conn = req.db_conn()?;

    let build_infos = BuildInfo::belonging_to(&version)
        .select(::version::build_info::BUILD_INFO_FIELDS)
        .load(&*conn)?;

    let mut encodable_build_info = EncodableVersionBuildInfo::default();
    encodable_build_info.id = version.id;
    let mut stables = BTreeSet::new();
    let mut betas = BTreeSet::new();
    let mut nightlies = BTreeSet::new();

    for row in build_infos {
        let BuildInfo {
            rust_version,
            target,
            passed,
            ..
        } = row;

        let rust_version = ParsedRustChannelVersion::from_str(&rust_version)?;

        match rust_version {
            ParsedRustChannelVersion::Stable(semver) => {
                let key = semver.to_string();
                stables.insert(semver);
                encodable_build_info
                    .stable
                    .entry(key)
                    .or_insert_with(HashMap::new)
                    .insert(target, passed);
            }
            ParsedRustChannelVersion::Beta(date) => {
                betas.insert(date);
                encodable_build_info
                    .beta
                    .entry(date)
                    .or_insert_with(HashMap::new)
                    .insert(target, passed);
            }
            ParsedRustChannelVersion::Nightly(date) => {
                nightlies.insert(date);
                encodable_build_info
                    .nightly
                    .entry(date)
                    .or_insert_with(HashMap::new)
                    .insert(target, passed);
            }
        }
    }

    encodable_build_info.ordering.insert(
        String::from("stable"),
        stables.into_iter().map(|s| s.to_string()).collect(),
    );

    fn naive_date_to_string(date: NaiveDate) -> String {
        date.format("%Y-%m-%d").to_string()
    }

    encodable_build_info.ordering.insert(
        String::from("beta"),
        betas.into_iter().map(naive_date_to_string).collect(),
    );

    encodable_build_info.ordering.insert(
        String::from("nightly"),
        nightlies.into_iter().map(naive_date_to_string).collect(),
    );

    #[derive(Serialize, Debug)]
    struct R {
        build_info: EncodableVersionBuildInfo,
    }

    Ok(req.json(&R {
        build_info: encodable_build_info,
    }))
}
