//! Endpoints that expose metadata about a crate
//!
//! These endpoints provide data that could be obtained direclty from the
//! index or cached metadata which was extracted (client side) from the
//! `Cargo.toml` file.

use conduit::{Request, Response};
use conduit_router::RequestParams;
use diesel::prelude::*;

use app::RequestApp;
use category::{CrateCategory, EncodableCategory};
use db::RequestTransaction;
use dependency::EncodableDependency;
use keyword::{CrateKeyword, EncodableKeyword};
use schema::*;
use util::{human, CargoResult, RequestUtils};
use version::EncodableVersion;
use {Category, Keyword, Version};

use super::{Crate, CrateDownload, EncodableCrate, ALL_COLUMNS};

/// Handles the `GET /summary` route.
pub fn summary(req: &mut Request) -> CargoResult<Response> {
    use diesel::dsl::*;
    use diesel::types::{BigInt, Nullable};
    use schema::crates::dsl::*;

    let conn = req.db_conn()?;
    let num_crates = crates.count().get_result(&*conn)?;
    let num_downloads = metadata::table
        .select(metadata::total_downloads)
        .get_result(&*conn)?;

    let encode_crates = |krates: Vec<Crate>| -> CargoResult<Vec<_>> {
        Version::belonging_to(&krates)
            .filter(versions::yanked.eq(false))
            .load::<Version>(&*conn)?
            .grouped_by(&krates)
            .into_iter()
            .map(|versions| Version::max(versions.into_iter().map(|v| v.num)))
            .zip(krates)
            .map(|(max_version, krate)| {
                Ok(krate.minimal_encodable(&max_version, None, false, None))
            })
            .collect()
    };

    let new_crates = crates
        .order(created_at.desc())
        .select(ALL_COLUMNS)
        .limit(10)
        .load(&*conn)?;
    let just_updated = crates
        .filter(updated_at.ne(created_at))
        .order(updated_at.desc())
        .select(ALL_COLUMNS)
        .limit(10)
        .load(&*conn)?;
    let most_downloaded = crates
        .order(downloads.desc())
        .select(ALL_COLUMNS)
        .limit(10)
        .load(&*conn)?;

    let recent_downloads = sql::<Nullable<BigInt>>("SUM(crate_downloads.downloads)");
    let most_recently_downloaded = crates
        .left_join(
            crate_downloads::table.on(
                id.eq(crate_downloads::crate_id)
                    .and(crate_downloads::date.gt(date(now - 90.days()))),
            ),
        )
        .group_by(id)
        .order(recent_downloads.desc().nulls_last())
        .limit(10)
        .select(ALL_COLUMNS)
        .load::<Crate>(&*conn)?;

    let popular_keywords = keywords::table
        .order(keywords::crates_cnt.desc())
        .limit(10)
        .load(&*conn)?
        .into_iter()
        .map(Keyword::encodable)
        .collect();

    let popular_categories = Category::toplevel(&conn, "crates", 10, 0)?
        .into_iter()
        .map(Category::encodable)
        .collect();

    #[derive(Serialize)]
    struct R {
        num_downloads: i64,
        num_crates: i64,
        new_crates: Vec<EncodableCrate>,
        most_downloaded: Vec<EncodableCrate>,
        most_recently_downloaded: Vec<EncodableCrate>,
        just_updated: Vec<EncodableCrate>,
        popular_keywords: Vec<EncodableKeyword>,
        popular_categories: Vec<EncodableCategory>,
    }
    Ok(req.json(&R {
        num_downloads: num_downloads,
        num_crates: num_crates,
        new_crates: encode_crates(new_crates)?,
        most_downloaded: encode_crates(most_downloaded)?,
        most_recently_downloaded: encode_crates(most_recently_downloaded)?,
        just_updated: encode_crates(just_updated)?,
        popular_keywords: popular_keywords,
        popular_categories: popular_categories,
    }))
}

/// Handles the `GET /crates/:crate_id` route.
pub fn show(req: &mut Request) -> CargoResult<Response> {
    use diesel::dsl::*;

    let name = &req.params()["crate_id"];
    let conn = req.db_conn()?;
    let krate = Crate::by_name(name).first::<Crate>(&*conn)?;

    let mut versions = Version::belonging_to(&krate).load::<Version>(&*conn)?;
    versions.sort_by(|a, b| b.num.cmp(&a.num));
    let ids = versions.iter().map(|v| v.id).collect();

    let kws = CrateKeyword::belonging_to(&krate)
        .inner_join(keywords::table)
        .select(keywords::all_columns)
        .load(&*conn)?;
    let cats = CrateCategory::belonging_to(&krate)
        .inner_join(categories::table)
        .select(categories::all_columns)
        .load(&*conn)?;
    let recent_downloads = CrateDownload::belonging_to(&krate)
        .filter(crate_downloads::date.gt(date(now - 90.days())))
        .select(sum(crate_downloads::downloads))
        .get_result(&*conn)?;

    let badges = badges::table
        .filter(badges::crate_id.eq(krate.id))
        .load(&*conn)?;
    let max_version = krate.max_version(&conn)?;

    #[derive(Serialize)]
    struct R {
        #[serde(rename = "crate")] krate: EncodableCrate,
        versions: Vec<EncodableVersion>,
        keywords: Vec<EncodableKeyword>,
        categories: Vec<EncodableCategory>,
    }
    Ok(
        req.json(&R {
            krate: krate.clone().encodable(
                &max_version,
                Some(ids),
                Some(&kws),
                Some(&cats),
                Some(badges),
                false,
                recent_downloads,
            ),
            versions: versions
                .into_iter()
                .map(|v| v.encodable(&krate.name))
                .collect(),
            keywords: kws.into_iter().map(|k| k.encodable()).collect(),
            categories: cats.into_iter().map(|k| k.encodable()).collect(),
        }),
    )
}

/// Handles the `GET /crates/:crate_id/:version/readme` route.
pub fn readme(req: &mut Request) -> CargoResult<Response> {
    let crate_name = &req.params()["crate_id"];
    let version = &req.params()["version"];

    let redirect_url = req.app()
        .config
        .uploader
        .readme_location(crate_name, version)
        .ok_or_else(|| human("crate readme not found"))?;

    if req.wants_json() {
        #[derive(Serialize)]
        struct R {
            url: String,
        }
        Ok(req.json(&R { url: redirect_url }))
    } else {
        Ok(req.redirect(redirect_url))
    }
}

/// Handles the `GET /crates/:crate_id/versions` route.
// FIXME: Not sure why this is necessary since /crates/:crate_id returns
// this information already, but ember is definitely requesting it
pub fn versions(req: &mut Request) -> CargoResult<Response> {
    let crate_name = &req.params()["crate_id"];
    let conn = req.db_conn()?;
    let krate = Crate::by_name(crate_name).first::<Crate>(&*conn)?;
    let mut versions = Version::belonging_to(&krate).load::<Version>(&*conn)?;
    versions.sort_by(|a, b| b.num.cmp(&a.num));
    let versions = versions
        .into_iter()
        .map(|v| v.encodable(crate_name))
        .collect();

    #[derive(Serialize)]
    struct R {
        versions: Vec<EncodableVersion>,
    }
    Ok(req.json(&R { versions: versions }))
}

/// Handles the `GET /crates/:crate_id/reverse_dependencies` route.
pub fn reverse_dependencies(req: &mut Request) -> CargoResult<Response> {
    use diesel::dsl::any;

    let name = &req.params()["crate_id"];
    let conn = req.db_conn()?;
    let krate = Crate::by_name(name).first::<Crate>(&*conn)?;
    let (offset, limit) = req.pagination(10, 100)?;
    let (rev_deps, total) = krate.reverse_dependencies(&*conn, offset, limit)?;
    let rev_deps: Vec<_> = rev_deps
        .into_iter()
        .map(|dep| dep.encodable(&krate.name))
        .collect();

    let version_ids: Vec<i32> = rev_deps.iter().map(|dep| dep.version_id).collect();

    let versions = versions::table
        .filter(versions::id.eq(any(version_ids)))
        .inner_join(crates::table)
        .select((versions::all_columns, crates::name))
        .load::<(Version, String)>(&*conn)?
        .into_iter()
        .map(|(version, krate_name)| version.encodable(&krate_name))
        .collect();

    #[derive(Serialize)]
    struct R {
        dependencies: Vec<EncodableDependency>,
        versions: Vec<EncodableVersion>,
        meta: Meta,
    }
    #[derive(Serialize)]
    struct Meta {
        total: i64,
    }
    Ok(req.json(&R {
        dependencies: rev_deps,
        versions,
        meta: Meta { total: total },
    }))
}
