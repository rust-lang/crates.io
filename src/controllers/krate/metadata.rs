//! Endpoints that expose metadata about a crate
//!
//! These endpoints provide data that could be obtained directly from the
//! index or cached metadata which was extracted (client side) from the
//! `Cargo.toml` file.

use std::cmp::Reverse;
use std::str::FromStr;

use crate::controllers::frontend_prelude::*;
use crate::controllers::helpers::pagination::PaginationOptions;

use crate::models::{
    Category, Crate, CrateCategory, CrateKeyword, CrateVersions, Keyword, RecentCrateDownloads,
    User, Version, VersionOwnerAction,
};
use crate::schema::*;
use crate::util::errors::crate_not_found;
use crate::views::{
    EncodableCategory, EncodableCrate, EncodableDependency, EncodableKeyword, EncodableVersion,
};

/// Handles the `GET /crates/new` special case.
pub async fn show_new(app: AppState, req: Parts) -> AppResult<Json<Value>> {
    show(app, Path("new".to_string()), req).await
}

/// Handles the `GET /crates/:crate_id` route.
pub async fn show(app: AppState, Path(name): Path<String>, req: Parts) -> AppResult<Json<Value>> {
    let conn = app.db_read_async().await?;
    conn.interact(move |conn| {
        let include = req
            .query()
            .get("include")
            .map(|mode| ShowIncludeMode::from_str(mode))
            .transpose()?
            .unwrap_or_default();

        let (krate, downloads): (Crate, i64) = Crate::by_name(&name)
            .inner_join(crate_downloads::table)
            .select((Crate::as_select(), crate_downloads::downloads))
            .first(conn)
            .optional()?
            .ok_or_else(|| crate_not_found(&name))?;

        let versions_publishers_and_audit_actions = if include.versions {
            let mut versions_and_publishers: Vec<(Version, Option<User>)> = krate
                .all_versions()
                .left_outer_join(users::table)
                .select((versions::all_columns, users::all_columns.nullable()))
                .load(conn)?;
            versions_and_publishers.sort_by_cached_key(|(version, _)| {
                Reverse(semver::Version::parse(&version.num).ok())
            });

            let versions = versions_and_publishers
                .iter()
                .map(|(v, _)| v)
                .cloned()
                .collect::<Vec<_>>();
            Some(
                versions_and_publishers
                    .into_iter()
                    .zip(VersionOwnerAction::for_versions(conn, &versions)?)
                    .map(|((v, pb), aas)| (v, pb, aas))
                    .collect::<Vec<_>>(),
            )
        } else {
            None
        };
        let ids = versions_publishers_and_audit_actions
            .as_ref()
            .map(|vps| vps.iter().map(|v| v.0.id).collect());

        let kws = if include.keywords {
            Some(
                CrateKeyword::belonging_to(&krate)
                    .inner_join(keywords::table)
                    .select(keywords::all_columns)
                    .load(conn)?,
            )
        } else {
            None
        };
        let cats = if include.categories {
            Some(
                CrateCategory::belonging_to(&krate)
                    .inner_join(categories::table)
                    .select(categories::all_columns)
                    .load(conn)?,
            )
        } else {
            None
        };
        let recent_downloads = if include.downloads {
            RecentCrateDownloads::belonging_to(&krate)
                .select(recent_crate_downloads::downloads)
                .get_result(conn)
                .optional()?
        } else {
            None
        };

        let badges = if include.badges { Some(vec![]) } else { None };

        let top_versions = if include.versions {
            Some(krate.top_versions(conn)?)
        } else {
            None
        };

        let encodable_crate = EncodableCrate::from(
            krate.clone(),
            top_versions.as_ref(),
            ids,
            kws.as_deref(),
            cats.as_deref(),
            badges,
            false,
            downloads,
            recent_downloads,
        );
        let encodable_versions = versions_publishers_and_audit_actions.map(|vpa| {
            vpa.into_iter()
                .map(|(v, pb, aas)| EncodableVersion::from(v, &krate.name, pb, aas))
                .collect::<Vec<_>>()
        });
        let encodable_keywords = kws.map(|kws| {
            kws.into_iter()
                .map(Keyword::into)
                .collect::<Vec<EncodableKeyword>>()
        });
        let encodable_cats = cats.map(|cats| {
            cats.into_iter()
                .map(Category::into)
                .collect::<Vec<EncodableCategory>>()
        });
        Ok(Json(json!({
            "crate": encodable_crate,
            "versions": encodable_versions,
            "keywords": encodable_keywords,
            "categories": encodable_cats,
        })))
    })
    .await?
}

#[derive(Debug)]
struct ShowIncludeMode {
    versions: bool,
    keywords: bool,
    categories: bool,
    badges: bool,
    downloads: bool,
}

impl Default for ShowIncludeMode {
    fn default() -> Self {
        // Send everything for legacy clients that expect the full response
        Self {
            versions: true,
            keywords: true,
            categories: true,
            badges: true,
            downloads: true,
        }
    }
}

impl ShowIncludeMode {
    const INVALID_COMPONENT: &'static str =
        "invalid component for ?include= (expected 'versions', 'keywords', 'categories', 'badges', 'downloads', or 'full')";
}

impl FromStr for ShowIncludeMode {
    type Err = BoxedAppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut mode = Self {
            versions: false,
            keywords: false,
            categories: false,
            badges: false,
            downloads: false,
        };
        for component in s.split(',') {
            match component {
                "" => {}
                "full" => {
                    mode = Self {
                        versions: true,
                        keywords: true,
                        categories: true,
                        badges: true,
                        downloads: true,
                    }
                }
                "versions" => mode.versions = true,
                "keywords" => mode.keywords = true,
                "categories" => mode.categories = true,
                "badges" => mode.badges = true,
                "downloads" => mode.downloads = true,
                _ => return Err(bad_request(Self::INVALID_COMPONENT)),
            }
        }
        Ok(mode)
    }
}

/// Handles the `GET /crates/:crate_id/:version/readme` route.
pub async fn readme(
    app: AppState,
    Path((crate_name, version)): Path<(String, String)>,
    req: Parts,
) -> Response {
    let redirect_url = app.storage.readme_location(&crate_name, &version);
    if req.wants_json() {
        Json(json!({ "url": redirect_url })).into_response()
    } else {
        redirect(redirect_url)
    }
}

/// Handles the `GET /crates/:crate_id/reverse_dependencies` route.
pub async fn reverse_dependencies(
    app: AppState,
    Path(name): Path<String>,
    req: Parts,
) -> AppResult<Json<Value>> {
    let conn = app.db_read_async().await?;
    conn.interact(move |conn| {
        let pagination_options = PaginationOptions::builder().gather(&req)?;

        let krate: Crate = Crate::by_name(&name)
            .first(conn)
            .optional()?
            .ok_or_else(|| crate_not_found(&name))?;

        let (rev_deps, total) = krate.reverse_dependencies(conn, pagination_options)?;
        let rev_deps: Vec<_> = rev_deps
            .into_iter()
            .map(|dep| EncodableDependency::from_reverse_dep(dep, &krate.name))
            .collect();

        let version_ids: Vec<i32> = rev_deps.iter().map(|dep| dep.version_id).collect();

        let versions_and_publishers: Vec<(Version, String, Option<User>)> = versions::table
            .filter(versions::id.eq_any(version_ids))
            .inner_join(crates::table)
            .left_outer_join(users::table)
            .select((
                versions::all_columns,
                crates::name,
                users::all_columns.nullable(),
            ))
            .load(conn)?;
        let versions = versions_and_publishers
            .iter()
            .map(|(v, _, _)| v)
            .cloned()
            .collect::<Vec<_>>();
        let versions = versions_and_publishers
            .into_iter()
            .zip(VersionOwnerAction::for_versions(conn, &versions)?)
            .map(|((version, krate_name, published_by), actions)| {
                EncodableVersion::from(version, &krate_name, published_by, actions)
            })
            .collect::<Vec<_>>();

        Ok(Json(json!({
            "dependencies": rev_deps,
            "versions": versions,
            "meta": { "total": total },
        })))
    })
    .await?
}
