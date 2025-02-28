//! Endpoint for exposing crate download counts
//!
//! The endpoint for downloading a crate and exposing version specific
//! download counts are located in `version::downloads`.

use crate::app::AppState;
use crate::controllers::krate::CratePath;
use crate::models::download::Version;
use crate::models::{User, Version as FullVersion, VersionDownload, VersionOwnerAction};
use crate::schema::{version_downloads, version_owner_actions, versions};
use crate::util::errors::{AppResult, BoxedAppError, bad_request};
use crate::views::{EncodableVersion, EncodableVersionDownload};
use axum::Json;
use axum::extract::FromRequestParts;
use axum_extra::extract::Query;
use crates_io_database::schema::users;
use crates_io_diesel_helpers::to_char;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use futures_util::FutureExt;
use futures_util::future::BoxFuture;
use std::cmp;
use std::str::FromStr;

#[derive(Debug, Deserialize, FromRequestParts, utoipa::IntoParams)]
#[from_request(via(Query))]
#[into_params(parameter_in = Query)]
pub struct DownloadsQueryParams {
    /// Additional data to include in the response.
    ///
    /// Valid values: `versions`.
    ///
    /// Defaults to no additional data.
    ///
    /// This parameter expects a comma-separated list of values.
    include: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DownloadsResponse {
    /// The per-day download counts for the last 90 days.
    pub version_downloads: Vec<EncodableVersionDownload>,

    /// The versions referenced in the download counts, if `?include=versions`
    /// was requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub versions: Option<Vec<EncodableVersion>>,

    #[schema(inline)]
    pub meta: DownloadsMeta,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DownloadsMeta {
    #[schema(inline)]
    pub extra_downloads: Vec<ExtraDownload>,
}

#[derive(Debug, Serialize, Queryable, utoipa::ToSchema)]
pub struct ExtraDownload {
    /// The date this download count is for.
    #[schema(example = "2019-12-13")]
    date: String,

    /// The number of downloads on the given date.
    #[schema(example = 123)]
    downloads: i64,
}

/// Get the download counts for a crate.
///
/// This includes the per-day downloads for the last 90 days and for the
/// latest 5 versions plus the sum of the rest.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/downloads",
    params(CratePath, DownloadsQueryParams),
    tag = "crates",
    responses((status = 200, description = "Successful Response", body = inline(DownloadsResponse))),
)]
pub async fn get_crate_downloads(
    state: AppState,
    path: CratePath,
    params: DownloadsQueryParams,
) -> AppResult<Json<DownloadsResponse>> {
    let mut conn = state.db_read().await?;

    use diesel::dsl::*;
    use diesel::sql_types::BigInt;

    let crate_id: i32 = path.load_crate_id(&mut conn).await?;

    let mut versions: Vec<Version> = versions::table
        .filter(versions::crate_id.eq(crate_id))
        .select(Version::as_select())
        .load(&mut conn)
        .await?;

    versions.sort_unstable_by(|a, b| b.num.cmp(&a.num));
    let (latest_five, rest) = versions.split_at(cmp::min(5, versions.len()));

    let include = params
        .include
        .as_ref()
        .map(|mode| ShowIncludeMode::from_str(mode))
        .transpose()?
        .unwrap_or_default();

    let sum_downloads = sql::<BigInt>("SUM(version_downloads.downloads)");
    let (downloads, extra_downloads, versions_and_publishers, actions) = tokio::try_join!(
        VersionDownload::belonging_to(latest_five)
            .filter(version_downloads::date.gt(date(now - 90.days())))
            .order((
                version_downloads::date.asc(),
                version_downloads::version_id.desc(),
            ))
            .load(&mut conn)
            .boxed(),
        VersionDownload::belonging_to(rest)
            .select((
                to_char(version_downloads::date, "YYYY-MM-DD"),
                sum_downloads,
            ))
            .filter(version_downloads::date.gt(date(now - 90.days())))
            .group_by(version_downloads::date)
            .order(version_downloads::date.asc())
            .load::<ExtraDownload>(&mut conn)
            .boxed(),
        load_versions_and_publishers(&mut conn, latest_five, include.versions),
        load_actions(&mut conn, latest_five, include.versions),
    )?;

    let version_downloads = downloads
        .into_iter()
        .map(VersionDownload::into)
        .collect::<Vec<EncodableVersionDownload>>();

    let versions = if include.versions {
        let versions_and_publishers = versions_and_publishers.grouped_by(latest_five);
        let actions = actions.grouped_by(latest_five);
        let versions = versions_and_publishers
            .into_iter()
            .zip(actions)
            .filter_map(|(vp, actions)| {
                vp.into_iter().next().map(|(version, publisher)| {
                    EncodableVersion::from(version, &path.name, publisher, actions)
                })
            })
            .collect::<Vec<_>>();

        Some(versions)
    } else {
        None
    };

    Ok(Json(DownloadsResponse {
        version_downloads,
        versions,
        meta: DownloadsMeta { extra_downloads },
    }))
}

type VersionsAndPublishers = (FullVersion, Option<User>);
fn load_versions_and_publishers<'a>(
    conn: &mut AsyncPgConnection,
    versions: &'a [Version],
    includes: bool,
) -> BoxFuture<'a, QueryResult<Vec<VersionsAndPublishers>>> {
    if !includes {
        return futures_util::future::always_ready(|| Ok(vec![])).boxed();
    }
    FullVersion::belonging_to(versions)
        .left_outer_join(users::table)
        .select(VersionsAndPublishers::as_select())
        .load(conn)
        .boxed()
}

fn load_actions<'a>(
    conn: &mut AsyncPgConnection,
    versions: &'a [Version],
    includes: bool,
) -> BoxFuture<'a, QueryResult<Vec<(VersionOwnerAction, User)>>> {
    if !includes {
        return futures_util::future::always_ready(|| Ok(vec![])).boxed();
    }
    VersionOwnerAction::belonging_to(versions)
        .inner_join(users::table)
        .order(version_owner_actions::id)
        .load(conn)
        .boxed()
}

#[derive(Debug, Default)]
struct ShowIncludeMode {
    versions: bool,
}

impl ShowIncludeMode {
    const INVALID_COMPONENT: &'static str = "invalid component for ?include= (expected 'versions')";
}

impl FromStr for ShowIncludeMode {
    type Err = BoxedAppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut mode = Self { versions: false };
        for component in s.split(',') {
            match component {
                "" => {}
                "versions" => mode.versions = true,
                _ => return Err(bad_request(Self::INVALID_COMPONENT)),
            }
        }
        Ok(mode)
    }
}
