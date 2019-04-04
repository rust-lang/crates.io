//! Endpoints that expose metadata about a crate
//!
//! These endpoints provide data that could be obtained direclty from the
//! index or cached metadata which was extracted (client side) from the
//! `Cargo.toml` file.

use crate::controllers::prelude::*;
use crate::models::{
    Category, Crate, CrateCategory, CrateKeyword, CrateVersions, Keyword, RecentCrateDownloads,
    User, Version,
};
use crate::schema::*;
use crate::views::{
    EncodableCategory, EncodableCrate, EncodableDependency, EncodableKeyword, EncodableVersion,
};

use crate::models::krate::ALL_COLUMNS;

/// Handles the `GET /summary` route.
pub fn summary(req: &mut dyn Request) -> CargoResult<Response> {
    use crate::schema::crates::dsl::*;

    let conn = req.db_conn()?;
    let num_crates = crates.count().get_result(&*conn)?;
    let num_downloads = metadata::table
        .select(metadata::total_downloads)
        .get_result(&*conn)?;

    let encode_crates = |krates: Vec<Crate>| -> CargoResult<Vec<_>> {
        let versions = krates.versions().load::<Version>(&*conn)?;
        versions
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

    let most_recently_downloaded = crates
        .inner_join(recent_crate_downloads::table)
        .order(recent_crate_downloads::downloads.desc())
        .select(ALL_COLUMNS)
        .limit(10)
        .load(&*conn)?;

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
        num_downloads,
        num_crates,
        new_crates: encode_crates(new_crates)?,
        most_downloaded: encode_crates(most_downloaded)?,
        most_recently_downloaded: encode_crates(most_recently_downloaded)?,
        just_updated: encode_crates(just_updated)?,
        popular_keywords,
        popular_categories,
    }))
}

/// Handles the `GET /crates/:crate_id` route.
pub fn show(req: &mut dyn Request) -> CargoResult<Response> {
    let name = &req.params()["crate_id"];
    let conn = req.db_conn()?;
    let krate = Crate::by_name(name).first::<Crate>(&*conn)?;

    let mut versions_and_publishers: Vec<(Version, Option<User>)> = krate
        .all_versions()
        .left_outer_join(users::table)
        .select((versions::all_columns, users::all_columns.nullable()))
        .load(&*conn)?;
    versions_and_publishers.sort_by(|a, b| b.0.num.cmp(&a.0.num));
    let ids = versions_and_publishers.iter().map(|v| v.0.id).collect();

    let kws = CrateKeyword::belonging_to(&krate)
        .inner_join(keywords::table)
        .select(keywords::all_columns)
        .load(&*conn)?;
    let cats = CrateCategory::belonging_to(&krate)
        .inner_join(categories::table)
        .select(categories::all_columns)
        .load(&*conn)?;
    let recent_downloads = RecentCrateDownloads::belonging_to(&krate)
        .select(recent_crate_downloads::downloads)
        .get_result(&*conn)
        .optional()?;

    let badges = badges::table
        .filter(badges::crate_id.eq(krate.id))
        .load(&*conn)?;
    let max_version = krate.max_version(&conn)?;

    #[derive(Serialize)]
    struct R {
        #[serde(rename = "crate")]
        krate: EncodableCrate,
        versions: Vec<EncodableVersion>,
        keywords: Vec<EncodableKeyword>,
        categories: Vec<EncodableCategory>,
    }
    Ok(req.json(&R {
        krate: krate.clone().encodable(
            &max_version,
            Some(ids),
            Some(&kws),
            Some(&cats),
            Some(badges),
            false,
            recent_downloads,
        ),
        versions: versions_and_publishers
            .into_iter()
            .map(|(v, pb)| v.encodable(&krate.name, pb))
            .collect(),
        keywords: kws.into_iter().map(Keyword::encodable).collect(),
        categories: cats.into_iter().map(Category::encodable).collect(),
    }))
}

/// Handles the `GET /crates/:crate_id/:version/readme` route.
pub fn readme(req: &mut dyn Request) -> CargoResult<Response> {
    let crate_name = &req.params()["crate_id"];
    let version = &req.params()["version"];

    let redirect_url = req
        .app()
        .config
        .uploader
        .readme_location(crate_name, version);

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
pub fn versions(req: &mut dyn Request) -> CargoResult<Response> {
    let crate_name = &req.params()["crate_id"];
    let conn = req.db_conn()?;
    let krate = Crate::by_name(crate_name).first::<Crate>(&*conn)?;
    let mut versions_and_publishers: Vec<(Version, Option<User>)> = krate
        .all_versions()
        .left_outer_join(users::table)
        .select((versions::all_columns, users::all_columns.nullable()))
        .load(&*conn)?;
    versions_and_publishers.sort_by(|a, b| b.0.num.cmp(&a.0.num));
    let versions = versions_and_publishers
        .into_iter()
        .map(|(v, pb)| v.encodable(crate_name, pb))
        .collect();

    #[derive(Serialize)]
    struct R {
        versions: Vec<EncodableVersion>,
    }
    Ok(req.json(&R { versions }))
}

/// Handles the `GET /crates/:crate_id/reverse_dependencies` route.
pub fn reverse_dependencies(req: &mut dyn Request) -> CargoResult<Response> {
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
        .left_outer_join(users::table)
        .select((
            versions::all_columns,
            crates::name,
            users::all_columns.nullable(),
        ))
        .load::<(Version, String, Option<User>)>(&*conn)?
        .into_iter()
        .map(|(version, krate_name, published_by)| version.encodable(&krate_name, published_by))
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
        meta: Meta { total },
    }))
}
