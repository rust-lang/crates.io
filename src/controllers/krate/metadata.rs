//! Endpoints that expose metadata about a crate
//!
//! These endpoints provide data that could be obtained directly from the
//! index or cached metadata which was extracted (client side) from the
//! `Cargo.toml` file.

use crate::app::AppState;
use crate::controllers::krate::CratePath;
use crate::models::{
    Category, Crate, CrateCategory, CrateKeyword, Keyword, RecentCrateDownloads, TopVersions, User,
    Version, VersionOwnerAction,
};
use crate::schema::*;
use crate::util::errors::{
    AppResult, BoxedAppError, bad_request, crate_not_found, version_not_found,
};
use crate::views::{EncodableCategory, EncodableCrate, EncodableKeyword, EncodableVersion};
use axum::Json;
use axum::extract::{FromRequestParts, Query};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use futures_util::FutureExt;
use futures_util::future::{BoxFuture, always_ready};
use std::str::FromStr;

#[derive(Debug, Deserialize, FromRequestParts, utoipa::IntoParams)]
#[from_request(via(Query))]
#[into_params(parameter_in = Query)]
pub struct FindQueryParams {
    /// Additional data to include in the response.
    ///
    /// Valid values: `versions`, `keywords`, `categories`, `badges`,
    /// `downloads`, `default_version`, or `full`.
    ///
    /// Defaults to `full` for backwards compatibility.
    ///
    /// **Note**: `versions` and `default_version` share the same key `versions`, therefore `default_version` will be ignored if both are provided.
    ///
    /// This parameter expects a comma-separated list of values.
    include: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct GetResponse {
    /// The crate metadata.
    #[serde(rename = "crate")]
    krate: EncodableCrate,

    /// The versions of the crate.
    #[schema(example = json!(null))]
    versions: Option<Vec<EncodableVersion>>,

    /// The keywords of the crate.
    #[schema(example = json!(null))]
    keywords: Option<Vec<EncodableKeyword>>,

    /// The categories of the crate.
    #[schema(example = json!(null))]
    categories: Option<Vec<EncodableCategory>>,
}

/// Get crate metadata (for the `new` crate).
///
/// This endpoint works around a small limitation in `axum` and is delegating
/// to the `GET /api/v1/crates/{name}` endpoint internally.
#[utoipa::path(
    get,
    path = "/api/v1/crates/new",
    tag = "crates",
    responses((status = 200, description = "Successful Response", body = inline(GetResponse))),
)]
pub async fn find_new_crate(
    app: AppState,
    params: FindQueryParams,
) -> AppResult<Json<GetResponse>> {
    let name = "new".to_string();
    find_crate(app, CratePath { name }, params).await
}

/// Get crate metadata.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}",
    params(CratePath, FindQueryParams),
    tag = "crates",
    responses((status = 200, description = "Successful Response", body = inline(GetResponse))),
)]
pub async fn find_crate(
    app: AppState,
    path: CratePath,
    params: FindQueryParams,
) -> AppResult<Json<GetResponse>> {
    let mut conn = app.db_read().await?;

    let include = params
        .include
        .map(|mode| ShowIncludeMode::from_str(&mode))
        .transpose()?
        .unwrap_or_default();

    let (krate, downloads, default_version, yanked, num_versions): (
        Crate,
        i64,
        Option<String>,
        Option<bool>,
        Option<i32>,
    ) = Crate::by_name(&path.name)
        .inner_join(crate_downloads::table)
        .left_join(default_versions::table)
        .left_join(versions::table.on(default_versions::version_id.eq(versions::id)))
        .select((
            Crate::as_select(),
            crate_downloads::downloads,
            versions::num.nullable(),
            versions::yanked.nullable(),
            default_versions::num_versions.nullable(),
        ))
        .first(&mut conn)
        .await
        .optional()?
        .ok_or_else(|| crate_not_found(&path.name))?;

    // Since `versions` and `default_version` share the same key (versions), we should only settle
    // the `include.default_version` when `include.versions` is not included, and ignore when no
    // `default_version` available.
    let include_default_version =
        include.default_version && !include.versions && default_version.is_some();
    let (versions_and_publishers, default_versions_and_publishers, kws, cats, recent_downloads) = tokio::try_join!(
        load_versions_and_publishers(&mut conn, &krate, include.versions),
        load_default_versions_and_publishers(
            &mut conn,
            &krate,
            default_version.as_deref(),
            include_default_version,
        ),
        load_keywords(&mut conn, &krate, include.keywords),
        load_categories(&mut conn, &krate, include.categories),
        load_recent_downloads(&mut conn, &krate, include.downloads),
    )?;

    let ids = versions_and_publishers
        .as_ref()
        .map(|vps| vps.iter().map(|v| v.0.id).collect());

    let versions_publishers_and_audit_actions = if let Some(versions_and_publishers) =
        versions_and_publishers.or(default_versions_and_publishers)
    {
        let versions = versions_and_publishers
            .iter()
            .map(|(v, _)| v)
            .collect::<Vec<_>>();
        let actions = VersionOwnerAction::for_versions(&mut conn, &versions).await?;
        Some(
            versions_and_publishers
                .into_iter()
                .zip(actions)
                .map(|((v, pb), aas)| (v, pb, aas))
                .collect::<Vec<_>>(),
        )
    } else {
        None
    };

    let top_versions = if let Some(versions) = versions_publishers_and_audit_actions
        .as_ref()
        .filter(|_| include.versions)
    {
        let pairs = versions
            .iter()
            .filter(|(v, _, _)| !v.yanked)
            .cloned()
            .map(|(v, _, _)| (v.created_at, v.num));
        Some(TopVersions::from_date_version_pairs(pairs))
    } else {
        None
    };

    let encodable_crate = EncodableCrate::from(
        krate.clone(),
        default_version.as_deref(),
        num_versions.unwrap_or_default(),
        yanked,
        top_versions.as_ref(),
        ids,
        kws.as_deref(),
        cats.as_deref(),
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

    Ok(Json(GetResponse {
        krate: encodable_crate,
        versions: encodable_versions,
        keywords: encodable_keywords,
        categories: encodable_cats,
    }))
}

type VersionsAndPublishers = (Version, Option<User>);

fn load_versions_and_publishers<'a>(
    conn: &mut AsyncPgConnection,
    krate: &'a Crate,
    includes: bool,
) -> BoxFuture<'a, AppResult<Option<Vec<VersionsAndPublishers>>>> {
    if !includes {
        return always_ready(|| Ok(None)).boxed();
    }

    _load_versions_and_publishers(conn, krate, None)
}

fn load_default_versions_and_publishers<'a>(
    conn: &mut AsyncPgConnection,
    krate: &'a Crate,
    version_num: Option<&'a str>,
    includes: bool,
) -> BoxFuture<'a, AppResult<Option<Vec<VersionsAndPublishers>>>> {
    if !includes || version_num.is_none() {
        return always_ready(|| Ok(None)).boxed();
    }

    let fut = _load_versions_and_publishers(conn, krate, version_num);
    async move {
        let records = fut.await?.ok_or_else(|| {
            version_not_found(
                &krate.name,
                version_num.expect("default_version should not be None"),
            )
        })?;
        Ok(Some(records))
    }
    .boxed()
}

fn load_keywords<'a>(
    conn: &mut AsyncPgConnection,
    krate: &'a Crate,
    includes: bool,
) -> BoxFuture<'a, AppResult<Option<Vec<Keyword>>>> {
    if !includes {
        return always_ready(|| Ok(None)).boxed();
    }

    let fut = CrateKeyword::belonging_to(&krate)
        .inner_join(keywords::table)
        .select(Keyword::as_select())
        .load(conn);
    async move { Ok(Some(fut.await?)) }.boxed()
}

fn load_categories<'a>(
    conn: &mut AsyncPgConnection,
    krate: &'a Crate,
    includes: bool,
) -> BoxFuture<'a, AppResult<Option<Vec<Category>>>> {
    if !includes {
        return always_ready(|| Ok(None)).boxed();
    }

    let fut = CrateCategory::belonging_to(&krate)
        .inner_join(categories::table)
        .select(Category::as_select())
        .load(conn);
    async move { Ok(Some(fut.await?)) }.boxed()
}

fn load_recent_downloads<'a>(
    conn: &mut AsyncPgConnection,
    krate: &'a Crate,
    includes: bool,
) -> BoxFuture<'a, AppResult<Option<i64>>> {
    if !includes {
        return always_ready(|| Ok(None)).boxed();
    }

    let fut = RecentCrateDownloads::belonging_to(&krate)
        .select(recent_crate_downloads::downloads)
        .get_result(conn);
    async move { Ok(fut.await.optional()?) }.boxed()
}

fn _load_versions_and_publishers<'a>(
    conn: &mut AsyncPgConnection,
    krate: &'a Crate,
    version_num: Option<&'a str>,
) -> BoxFuture<'a, AppResult<Option<Vec<VersionsAndPublishers>>>> {
    let mut query = Version::belonging_to(&krate)
        .left_outer_join(users::table)
        .select(<(Version, Option<User>)>::as_select())
        .order_by(versions::id.desc())
        .into_boxed();

    if let Some(num) = version_num {
        query = query.filter(versions::num.eq(num));
    }

    let fut = query.load(conn);
    async move { Ok(Some(fut.await?)) }.boxed()
}

#[derive(Debug)]
struct ShowIncludeMode {
    versions: bool,
    keywords: bool,
    categories: bool,
    badges: bool,
    downloads: bool,
    default_version: bool,
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
            default_version: true,
        }
    }
}

impl ShowIncludeMode {
    const INVALID_COMPONENT: &'static str = "invalid component for ?include= (expected 'versions', 'keywords', 'categories', 'badges', 'downloads', 'default_version', or 'full')";
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
            default_version: false,
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
                        default_version: true,
                    }
                }
                "versions" => mode.versions = true,
                "keywords" => mode.keywords = true,
                "categories" => mode.categories = true,
                "badges" => mode.badges = true,
                "downloads" => mode.downloads = true,
                "default_version" => mode.default_version = true,
                _ => return Err(bad_request(Self::INVALID_COMPONENT)),
            }
        }
        Ok(mode)
    }
}
