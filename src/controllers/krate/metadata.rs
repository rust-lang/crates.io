//! Endpoints that expose metadata about a crate
//!
//! These endpoints provide data that could be obtained directly from the
//! index or cached metadata which was extracted (client side) from the
//! `Cargo.toml` file.

use std::cmp::Reverse;

use crate::controllers::frontend_prelude::*;
use crate::controllers::helpers::pagination::PaginationOptions;

use crate::models::{
    Category, Crate, CrateCategory, CrateKeyword, CrateVersions, Keyword, RecentCrateDownloads,
    TopVersions, User, Version, VersionOwnerAction,
};
use crate::schema::*;
use crate::views::{
    EncodableCategory, EncodableCrate, EncodableDependency, EncodableKeyword, EncodableVersion,
};

use crate::models::krate::ALL_COLUMNS;

/// Handles the `GET /summary` route.
pub fn summary(req: &mut dyn RequestExt) -> EndpointResult {
    use crate::schema::crates::dsl::*;

    let conn = req.db_read_only()?;
    let num_crates: i64 = crates.count().get_result(&*conn)?;
    let num_downloads: i64 = metadata::table
        .select(metadata::total_downloads)
        .get_result(&*conn)?;

    let encode_crates = |data: Vec<(Crate, Option<i64>)>| -> AppResult<Vec<_>> {
        let recent_downloads = data.iter().map(|&(_, s)| s).collect::<Vec<_>>();

        let krates = data.into_iter().map(|(c, _)| c).collect::<Vec<_>>();

        let versions: Vec<Version> = krates.versions().load(&*conn)?;
        versions
            .grouped_by(&krates)
            .into_iter()
            .map(TopVersions::from_versions)
            .zip(krates)
            .zip(recent_downloads)
            .map(|((top_versions, krate), recent_downloads)| {
                Ok(EncodableCrate::from_minimal(
                    krate,
                    &top_versions,
                    None,
                    false,
                    recent_downloads,
                ))
            })
            .collect()
    };

    let selection = (ALL_COLUMNS, recent_crate_downloads::downloads.nullable());

    let new_crates = crates
        .left_join(recent_crate_downloads::table)
        .order(created_at.desc())
        .select(selection)
        .limit(10)
        .load(&*conn)?;
    let just_updated = crates
        .left_join(recent_crate_downloads::table)
        .filter(updated_at.ne(created_at))
        .order(updated_at.desc())
        .select(selection)
        .limit(10)
        .load(&*conn)?;
    let most_downloaded = crates
        .left_join(recent_crate_downloads::table)
        .then_order_by(downloads.desc())
        .select(selection)
        .limit(10)
        .load(&*conn)?;

    let most_recently_downloaded = crates
        .inner_join(recent_crate_downloads::table)
        .then_order_by(recent_crate_downloads::downloads.desc())
        .select(selection)
        .limit(10)
        .load(&*conn)?;

    let popular_keywords = keywords::table
        .order(keywords::crates_cnt.desc())
        .limit(10)
        .load(&*conn)?
        .into_iter()
        .map(Keyword::into)
        .collect::<Vec<EncodableKeyword>>();

    let popular_categories = Category::toplevel(&conn, "crates", 10, 0)?
        .into_iter()
        .map(Category::into)
        .collect::<Vec<EncodableCategory>>();

    Ok(req.json(&json!({
        "num_downloads": num_downloads,
        "num_crates": num_crates,
        "new_crates": encode_crates(new_crates)?,
        "most_downloaded": encode_crates(most_downloaded)?,
        "most_recently_downloaded": encode_crates(most_recently_downloaded)?,
        "just_updated": encode_crates(just_updated)?,
        "popular_keywords": popular_keywords,
        "popular_categories": popular_categories,
    })))
}

/// Handles the `GET /crates/:crate_id` route.
pub fn show(req: &mut dyn RequestExt) -> EndpointResult {
    let name = &req.params()["crate_id"];
    let conn = req.db_read_only()?;
    let krate: Crate = Crate::by_name(name).first(&*conn)?;

    let mut versions_and_publishers: Vec<(Version, Option<User>)> = krate
        .all_versions()
        .left_outer_join(users::table)
        .select((versions::all_columns, users::all_columns.nullable()))
        .load(&*conn)?;

    versions_and_publishers
        .sort_by_cached_key(|(version, _)| Reverse(semver::Version::parse(&version.num).ok()));

    let versions = versions_and_publishers
        .iter()
        .map(|(v, _)| v)
        .cloned()
        .collect::<Vec<_>>();
    let versions_publishers_and_audit_actions = versions_and_publishers
        .into_iter()
        .zip(VersionOwnerAction::for_versions(&conn, &versions)?.into_iter())
        .map(|((v, pb), aas)| (v, pb, aas))
        .collect::<Vec<_>>();
    let ids = versions_publishers_and_audit_actions
        .iter()
        .map(|v| v.0.id)
        .collect();

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
    let top_versions = krate.top_versions(&conn)?;

    Ok(req.json(&json!({
        "crate": EncodableCrate::from(
            krate.clone(),
            &top_versions,
            Some(ids),
            Some(&kws),
            Some(&cats),
            Some(badges),
            false,
            recent_downloads,
        ),
        "versions": versions_publishers_and_audit_actions
            .into_iter()
            .map(|(v, pb, aas)| EncodableVersion::from(v, &krate.name, pb, aas))
            .collect::<Vec<_>>(),
        "keywords": kws.into_iter().map(Keyword::into).collect::<Vec<EncodableKeyword>>(),
        "categories": cats.into_iter().map(Category::into).collect::<Vec<EncodableCategory>>(),
    })))
}

/// Handles the `GET /crates/:crate_id/:version/readme` route.
pub fn readme(req: &mut dyn RequestExt) -> EndpointResult {
    let crate_name = &req.params()["crate_id"];
    let version = &req.params()["version"];

    let redirect_url = req
        .app()
        .config
        .uploader()
        .readme_location(crate_name, version);

    if req.wants_json() {
        Ok(req.json(&json!({ "url": redirect_url })))
    } else {
        Ok(req.redirect(redirect_url))
    }
}

/// Handles the `GET /crates/:crate_id/versions` route.
// FIXME: Not sure why this is necessary since /crates/:crate_id returns
// this information already, but ember is definitely requesting it
pub fn versions(req: &mut dyn RequestExt) -> EndpointResult {
    let crate_name = &req.params()["crate_id"];
    let conn = req.db_read_only()?;
    let krate: Crate = Crate::by_name(crate_name).first(&*conn)?;
    let mut versions_and_publishers: Vec<(Version, Option<User>)> = krate
        .all_versions()
        .left_outer_join(users::table)
        .select((versions::all_columns, users::all_columns.nullable()))
        .load(&*conn)?;

    versions_and_publishers
        .sort_by_cached_key(|(version, _)| Reverse(semver::Version::parse(&version.num).ok()));

    let versions = versions_and_publishers
        .iter()
        .map(|(v, _)| v)
        .cloned()
        .collect::<Vec<_>>();
    let versions = versions_and_publishers
        .into_iter()
        .zip(VersionOwnerAction::for_versions(&conn, &versions)?.into_iter())
        .map(|((v, pb), aas)| EncodableVersion::from(v, crate_name, pb, aas))
        .collect::<Vec<_>>();

    Ok(req.json(&json!({ "versions": versions })))
}

/// Handles the `GET /crates/:crate_id/reverse_dependencies` route.
pub fn reverse_dependencies(req: &mut dyn RequestExt) -> EndpointResult {
    use diesel::dsl::any;

    let pagination_options = PaginationOptions::builder().gather(req)?;
    let name = &req.params()["crate_id"];
    let conn = req.db_read_only()?;
    let krate: Crate = Crate::by_name(name).first(&*conn)?;
    let (rev_deps, total) = krate.reverse_dependencies(&*conn, pagination_options)?;
    let rev_deps: Vec<_> = rev_deps
        .into_iter()
        .map(|dep| EncodableDependency::from_reverse_dep(dep, &krate.name))
        .collect();

    let version_ids: Vec<i32> = rev_deps.iter().map(|dep| dep.version_id).collect();

    let versions_and_publishers: Vec<(Version, String, Option<User>)> = versions::table
        .filter(versions::id.eq(any(version_ids)))
        .inner_join(crates::table)
        .left_outer_join(users::table)
        .select((
            versions::all_columns,
            crates::name,
            users::all_columns.nullable(),
        ))
        .load(&*conn)?;
    let versions = versions_and_publishers
        .iter()
        .map(|(v, _, _)| v)
        .cloned()
        .collect::<Vec<_>>();
    let versions = versions_and_publishers
        .into_iter()
        .zip(VersionOwnerAction::for_versions(&conn, &versions)?.into_iter())
        .map(|((version, krate_name, published_by), actions)| {
            EncodableVersion::from(version, &krate_name, published_by, actions)
        })
        .collect::<Vec<_>>();

    Ok(req.json(&json!({
        "dependencies": rev_deps,
        "versions": versions,
        "meta": { "total": total },
    })))
}
