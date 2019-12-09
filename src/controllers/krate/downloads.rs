//! Endpoint for exposing crate download counts
//!
//! The enpoints for download a crate and exposing version specific
//! download counts are located in `krate::downloads`.

use std::cmp;

use crate::controllers::prelude::*;

use crate::models::{Crate, CrateVersions, Version, VersionDownload};
use crate::schema::version_downloads;
use crate::views::EncodableVersionDownload;

use crate::models::krate::to_char;

/// Handles the `GET /crates/:crate_id/downloads` route.
pub fn downloads(req: &mut dyn Request) -> AppResult<Response> {
    use diesel::dsl::*;
    use diesel::sql_types::BigInt;

    let crate_name = &req.params()["crate_id"];
    let conn = req.db_conn()?;
    let krate = Crate::by_name(crate_name).first::<Crate>(&*conn)?;

    let mut versions = krate.all_versions().load::<Version>(&*conn)?;
    versions.sort_by(|a, b| b.num.cmp(&a.num));
    let (latest_five, rest) = versions.split_at(cmp::min(5, versions.len()));

    let downloads = VersionDownload::belonging_to(latest_five)
        .filter(version_downloads::date.gt(date(now - 90.days())))
        .order(version_downloads::date.asc())
        .load(&*conn)?
        .into_iter()
        .map(VersionDownload::encodable)
        .collect::<Vec<_>>();

    let sum_downloads = sql::<BigInt>("SUM(version_downloads.downloads)");
    let extra = VersionDownload::belonging_to(rest)
        .select((
            to_char(version_downloads::date, "YYYY-MM-DD"),
            sum_downloads,
        ))
        .filter(version_downloads::date.gt(date(now - 90.days())))
        .group_by(version_downloads::date)
        .order(version_downloads::date.asc())
        .load::<ExtraDownload>(&*conn)?;

    #[derive(Serialize, Queryable)]
    struct ExtraDownload {
        date: String,
        downloads: i64,
    }
    #[derive(Serialize)]
    struct R {
        version_downloads: Vec<EncodableVersionDownload>,
        meta: Meta,
    }
    #[derive(Serialize)]
    struct Meta {
        extra_downloads: Vec<ExtraDownload>,
    }
    let meta = Meta {
        extra_downloads: extra,
    };
    Ok(req.json(&R {
        version_downloads: downloads,
        meta,
    }))
}

/// Handles the `GET /crates/:crate_id/recent_downloads` route.
pub fn recent_downloads(req: &mut dyn Request) -> AppResult<Response> {
    use diesel::dsl::*;

    let ndays = 90;

    let crate_name = &req.params()["crate_id"];
    let conn = req.db_conn()?;
    let krate = Crate::by_name(crate_name).first::<Crate>(&*conn)?;

    // Get the versions for this crate
    let versions = krate.all_versions().load::<Version>(&*conn)?;

    #[derive(Debug, Serialize)]
    struct Download<'a> {
        version: &'a semver::Version,
        downloads: i32,
    }

    #[derive(Debug, Serialize)]
    struct Response<'a> {
        downloads: Vec<Download<'a>>,
        meta: Meta<'a>,
    }

    #[derive(Debug, Serialize)]
    struct Meta<'a> {
        #[serde(rename = "crate")]
        krate: &'a str,
        ndays: i32,
    }

    // Now get the grouped versions for the last `ndays` days.
    //
    // XXX I am not sure how to do this in the database yet, with Diesel's API, so for the time
    // being, perform this aggregation in Rust.
    let downloads = VersionDownload::belonging_to(versions.as_slice())
        .filter(version_downloads::date.gt(date(now - ndays.days())))
        .load(&*conn)?
        .grouped_by(versions.as_slice())
        .into_iter()
        .map(|grouped_versions: Vec<VersionDownload>| {
            let total_downloads = grouped_versions.iter().map(|v| v.downloads).sum();
            // XXX(perf) this is slow, iterating over the versions array looking for the matching
            // version.
            let version = versions
                .iter()
                .find(|v| v.id == grouped_versions[0].version_id)
                .unwrap();

            Download {
                version: &version.num,
                downloads: total_downloads,
            }
        })
        .collect::<Vec<Download<'_>>>();

    Ok(req.json(&Response {
        downloads,
        meta: Meta {
            krate: crate_name,
            ndays,
        },
    }))
}
