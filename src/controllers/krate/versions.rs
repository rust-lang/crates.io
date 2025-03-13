//! Endpoint for versions of a crate

use crate::app::AppState;
use crate::controllers::helpers::pagination::{
    Page, PaginationOptions, PaginationQueryParams, encode_seek,
};
use crate::controllers::krate::CratePath;
use crate::models::{User, Version, VersionOwnerAction};
use crate::schema::{users, versions};
use crate::util::RequestUtils;
use crate::util::errors::{AppResult, BoxedAppError, bad_request};
use crate::util::string_excl_null::StringExclNull;
use crate::views::EncodableVersion;
use axum::Json;
use axum::extract::FromRequestParts;
use axum_extra::extract::Query;
use diesel::dsl::not;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use futures_util::{TryStreamExt, future};
use http::request::Parts;
use indexmap::{IndexMap, IndexSet};
use std::str::FromStr;

#[derive(Debug, Deserialize, FromRequestParts, utoipa::IntoParams)]
#[from_request(via(Query))]
#[into_params(parameter_in = Query)]
pub struct ListQueryParams {
    /// Additional data to include in the response.
    ///
    /// Valid values: `release_tracks`.
    ///
    /// Defaults to no additional data.
    ///
    /// This parameter expects a comma-separated list of values.
    include: Option<String>,

    /// The sort order of the versions.
    ///
    /// Valid values: `date`, and `semver`.
    ///
    /// Defaults to `semver`.
    sort: Option<String>,

    /// If set, only versions with the specified semver strings are returned.
    #[serde(rename = "nums[]", default)]
    #[param(inline)]
    nums: Vec<StringExclNull>,
}

impl ListQueryParams {
    fn include(&self) -> AppResult<ShowIncludeMode> {
        let include = self
            .include
            .as_ref()
            .map(|mode| ShowIncludeMode::from_str(mode))
            .transpose()?
            .unwrap_or_default();
        Ok(include)
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ListResponse {
    versions: Vec<EncodableVersion>,

    #[schema(inline)]
    meta: ResponseMeta,
}

/// List all versions of a crate.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/versions",
    params(CratePath, ListQueryParams, PaginationQueryParams),
    tag = "versions",
    responses((status = 200, description = "Successful Response", body = inline(ListResponse))),
)]
pub async fn list_versions(
    state: AppState,
    path: CratePath,
    params: ListQueryParams,
    pagination: PaginationQueryParams,
    req: Parts,
) -> AppResult<Json<ListResponse>> {
    let mut conn = state.db_read().await?;

    let crate_id = path.load_crate_id(&mut conn).await?;

    // To keep backward compatibility, we paginate only if per_page is provided
    let pagination = match pagination.per_page {
        Some(_) => Some(
            PaginationOptions::builder()
                .enable_seek(true)
                .enable_pages(false)
                .gather(&req)?,
        ),
        None => None,
    };

    // Sort by semver by default
    let versions_and_publishers = match &params.sort.as_ref().map(|s| s.to_lowercase()).as_deref() {
        Some("date") => {
            list_by_date(crate_id, pagination.as_ref(), &params, &req, &mut conn).await?
        }
        _ => list_by_semver(crate_id, pagination.as_ref(), &params, &req, &mut conn).await?,
    };

    let versions = versions_and_publishers
        .data
        .iter()
        .map(|(v, _)| v)
        .collect::<Vec<_>>();
    let actions = VersionOwnerAction::for_versions(&mut conn, &versions).await?;
    let versions = versions_and_publishers
        .data
        .into_iter()
        .zip(actions)
        .map(|((v, pb), aas)| EncodableVersion::from(v, &path.name, pb, aas))
        .collect::<Vec<_>>();

    Ok(Json(ListResponse {
        versions,
        meta: versions_and_publishers.meta,
    }))
}

/// Seek-based pagination of versions by date
///
/// # Panics
///
/// This function will panic if `option` is built with `enable_pages` set to true.
async fn list_by_date(
    crate_id: i32,
    options: Option<&PaginationOptions>,
    params: &ListQueryParams,
    req: &Parts,
    conn: &mut AsyncPgConnection,
) -> AppResult<PaginatedVersionsAndPublishers> {
    use seek::*;

    let make_base_query = || {
        let mut query = versions::table
            .filter(versions::crate_id.eq(crate_id))
            .left_outer_join(users::table)
            .select(<(Version, Option<User>)>::as_select())
            .into_boxed();

        if !params.nums.is_empty() {
            query = query.filter(versions::num.eq_any(params.nums.iter().map(|s| s.as_str())));
        }
        query
    };

    let mut query = make_base_query();

    if let Some(options) = options {
        assert!(!options.is_explicit(), "?page= is not supported");
        if let Some(SeekPayload::Date(Date { created_at, id })) =
            Seek::Date.decode(&options.page)?
        {
            query = query.filter(
                versions::created_at
                    .eq(created_at)
                    .and(versions::id.lt(id))
                    .or(versions::created_at.lt(created_at)),
            )
        }
        query = query.limit(options.per_page);
    }

    query = query.order((versions::created_at.desc(), versions::id.desc()));

    let data: Vec<(Version, Option<User>)> = query.load(conn).await?;
    let mut next_page = None;
    if let Some(options) = options {
        next_page = next_seek_params(&data, options, |last| Seek::Date.to_payload(last))?
            .map(|p| req.query_with_params(p));
    };

    let release_tracks = if params.include()?.release_tracks {
        let mut sorted_versions = IndexSet::new();
        if options.is_some() {
            versions::table
                .filter(versions::crate_id.eq(crate_id))
                .filter(not(versions::yanked))
                .select(versions::num)
                .load_stream::<String>(conn)
                .await?
                .try_for_each(|num| {
                    if let Ok(semver) = semver::Version::parse(&num) {
                        sorted_versions.insert(semver);
                    };
                    future::ready(Ok(()))
                })
                .await?;
        } else {
            sorted_versions = data
                .iter()
                .flat_map(|(version, _)| {
                    (!version.yanked)
                        .then_some(version)
                        .and_then(|v| semver::Version::parse(&v.num).ok())
                })
                .collect();
        }

        sorted_versions.sort_unstable_by(|a, b| b.cmp(a));
        Some(ReleaseTracks::from_sorted_semver_iter(
            sorted_versions.iter(),
        ))
    } else {
        None
    };

    // Since the total count is retrieved through an additional query, to maintain consistency
    // with other pagination methods, we only make a count query while data is not empty.
    let total = if !data.is_empty() {
        make_base_query().count().get_result(conn).await?
    } else {
        0
    };

    Ok(PaginatedVersionsAndPublishers {
        data,
        meta: ResponseMeta {
            total,
            next_page,
            release_tracks,
        },
    })
}

/// Seek-based pagination of versions by semver
///
/// Unfortunately, Heroku Postgres has no support for the semver PG extension.
/// Therefore, we need to perform both sorting and pagination manually on the server.
///
/// # Panics
///
/// This function will panic if `option` is built with `enable_pages` set to true.
async fn list_by_semver(
    crate_id: i32,
    options: Option<&PaginationOptions>,
    params: &ListQueryParams,
    req: &Parts,
    conn: &mut AsyncPgConnection,
) -> AppResult<PaginatedVersionsAndPublishers> {
    use seek::*;

    let include = params.include()?;
    let mut query = versions::table
        .filter(versions::crate_id.eq(crate_id))
        .into_boxed();

    if !params.nums.is_empty() {
        query = query.filter(versions::num.eq_any(params.nums.iter().map(|s| s.as_str())));
    }

    let (data, total, release_tracks) = if let Some(options) = options {
        // Since versions will only increase in the future and both sorting and pagination need to
        // happen on the app server, implementing it with fetching only the data needed for sorting
        // and pagination, then making another query for the data to respond with, would minimize
        // payload and memory usage. This way, we can utilize the sorted map and enrich it later
        // without sorting twice.
        // Sorting by semver but opted for id as the seek key because num can be quite lengthy,
        // while id values are significantly smaller.

        let mut sorted_versions = IndexMap::new();
        query
            .select((versions::id, versions::num, versions::yanked))
            .load_stream::<(i32, String, bool)>(conn)
            .await?
            .try_for_each(|(id, num, yanked)| {
                let semver = semver::Version::parse(&num).ok();
                sorted_versions.insert(id, (semver, yanked, None));
                future::ready(Ok(()))
            })
            .await?;

        sorted_versions
            .sort_unstable_by(|_, (semver_a, _, _), _, (semver_b, _, _)| semver_b.cmp(semver_a));

        assert!(!options.is_explicit(), "?page= is not supported");

        let release_tracks = include.release_tracks.then(|| {
            ReleaseTracks::from_sorted_semver_iter(
                sorted_versions
                    .values()
                    .filter(|(_, yanked, _)| !yanked)
                    .filter_map(|(semver, _, _)| semver.as_ref()),
            )
        });

        let mut idx = Some(0);
        if let Some(SeekPayload::Semver(Semver { id })) = Seek::Semver.decode(&options.page)? {
            idx = sorted_versions
                .get_index_of(&id)
                .filter(|i| i + 1 < sorted_versions.len())
                .map(|i| i + 1);
        }
        if let Some(start) = idx {
            let end = (start + options.per_page as usize).min(sorted_versions.len());
            let ids = sorted_versions[start..end]
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            versions::table
                .filter(versions::crate_id.eq(crate_id))
                .left_outer_join(users::table)
                .select(<(Version, Option<User>)>::as_select())
                .filter(versions::id.eq_any(ids))
                .load_stream::<(Version, Option<User>)>(conn)
                .await?
                .try_for_each(|row| {
                    // The versions are already sorted, and we only need to enrich the fetched rows into them.
                    // Therefore, other values can now be safely ignored.
                    sorted_versions
                        .entry(row.0.id)
                        .and_modify(|entry| *entry = (None, false, Some(row)));

                    future::ready(Ok(()))
                })
                .await?;

            let len = sorted_versions.len();
            (
                sorted_versions
                    .into_values()
                    .filter_map(|(_, _, v)| v)
                    .collect(),
                len,
                release_tracks,
            )
        } else {
            (vec![], 0, release_tracks)
        }
    } else {
        let mut data = IndexMap::new();
        query
            .left_outer_join(users::table)
            .select(<(Version, Option<User>)>::as_select())
            .load_stream::<(Version, Option<User>)>(conn)
            .await?
            .try_for_each(|row| {
                if let Ok(semver) = semver::Version::parse(&row.0.num) {
                    data.insert(semver, row);
                };
                future::ready(Ok(()))
            })
            .await?;
        data.sort_unstable_by(|a, _, b, _| b.cmp(a));
        let total = data.len();
        let release_tracks = include.release_tracks.then(|| {
            ReleaseTracks::from_sorted_semver_iter(
                data.iter()
                    .flat_map(|(semver, (version, _))| (!version.yanked).then_some(semver)),
            )
        });
        (data.into_values().collect(), total, release_tracks)
    };

    let mut next_page = None;
    if let Some(options) = options {
        next_page = next_seek_params(&data, options, |last| Seek::Semver.to_payload(last))?
            .map(|p| req.query_with_params(p))
    };

    Ok(PaginatedVersionsAndPublishers {
        data,
        meta: ResponseMeta {
            total: total as i64,
            next_page,
            release_tracks,
        },
    })
}

mod seek {
    use crate::controllers::helpers::pagination::seek;
    use crate::models::{User, Version};
    use chrono::Utc;
    use chrono::serde::ts_microseconds;

    // We might consider refactoring this to use named fields, which would be clearer and more
    // flexible. It's also worth noting that we currently encode seek compactly as a Vec, which
    // doesn't include field names.
    seek!(
        pub enum Seek {
            Semver {
                id: i32,
            },
            Date {
                #[serde(with = "ts_microseconds")]
                created_at: chrono::DateTime<Utc>,
                id: i32,
            },
        }
    );

    impl Seek {
        pub(crate) fn to_payload(&self, record: &(Version, Option<User>)) -> SeekPayload {
            let (Version { id, created_at, .. }, _) = *record;
            match *self {
                Seek::Semver => SeekPayload::Semver(Semver { id }),
                Seek::Date => SeekPayload::Date(Date { created_at, id }),
            }
        }
    }
}

fn next_seek_params<T, S, F>(
    records: &[T],
    options: &PaginationOptions,
    f: F,
) -> AppResult<Option<IndexMap<String, String>>>
where
    F: Fn(&T) -> S,
    S: serde::Serialize,
{
    if options.is_explicit() || records.len() < options.per_page as usize {
        return Ok(None);
    }

    let mut opts = IndexMap::new();
    match options.page {
        Page::Unspecified | Page::Seek(_) => {
            let seek = f(records.last().unwrap());
            opts.insert("seek".into(), encode_seek(seek)?);
        }
        Page::Numeric(_) | Page::SeekBackward(_) => unreachable!(),
    };
    Ok(Some(opts))
}

struct PaginatedVersionsAndPublishers {
    data: Vec<(Version, Option<User>)>,
    meta: ResponseMeta,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
struct ResponseMeta {
    /// The total number of versions belonging to the crate.
    #[schema(example = 123)]
    total: i64,

    /// Query string to the next page of results, if any.
    #[schema(example = "?page=3")]
    next_page: Option<String>,

    /// Additional data about the crate's release tracks,
    /// if `?include=release_tracks` is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    release_tracks: Option<ReleaseTracks>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
struct ReleaseTracks(IndexMap<ReleaseTrackName, ReleaseTrackDetails>);

impl ReleaseTracks {
    // Return the release tracks based on a sorted semver versions iterator (in descending order).
    // **Remember to** filter out yanked versions manually before calling this function.
    pub fn from_sorted_semver_iter<'a, I>(versions: I) -> Self
    where
        I: Iterator<Item = &'a semver::Version>,
    {
        let mut map = IndexMap::new();
        for num in versions.filter(|num| num.pre.is_empty()) {
            let key = ReleaseTrackName::from_semver(num);
            let prev = map.last();
            if prev.filter(|&(k, _)| *k == key).is_none() {
                map.insert(
                    key,
                    ReleaseTrackDetails {
                        highest: num.clone(),
                    },
                );
            }
        }

        Self(map)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum ReleaseTrackName {
    Minor(u64),
    Major(u64),
}

impl ReleaseTrackName {
    pub fn from_semver(version: &semver::Version) -> Self {
        if version.major == 0 {
            Self::Minor(version.minor)
        } else {
            Self::Major(version.major)
        }
    }
}

impl std::fmt::Display for ReleaseTrackName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Minor(minor) => write!(f, "0.{minor}"),
            Self::Major(major) => write!(f, "{major}"),
        }
    }
}

impl serde::Serialize for ReleaseTrackName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        Self: std::fmt::Display,
    {
        serializer.collect_str(self)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
struct ReleaseTrackDetails {
    highest: semver::Version,
}

#[derive(Debug, Default)]
struct ShowIncludeMode {
    release_tracks: bool,
}

impl ShowIncludeMode {
    const INVALID_COMPONENT: &'static str =
        "invalid component for ?include= (expected 'release_tracks')";
}

impl FromStr for ShowIncludeMode {
    type Err = BoxedAppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut mode = Self {
            release_tracks: false,
        };
        for component in s.split(',') {
            match component {
                "" => {}
                "release_tracks" => mode.release_tracks = true,
                _ => return Err(bad_request(Self::INVALID_COMPONENT)),
            }
        }
        Ok(mode)
    }
}

#[cfg(test)]
mod tests {
    use super::{ReleaseTrackDetails, ReleaseTrackName, ReleaseTracks};
    use indexmap::IndexMap;
    use serde_json::json;

    #[track_caller]
    fn version(str: &str) -> semver::Version {
        semver::Version::parse(str).unwrap()
    }

    #[test]
    fn release_tracks_empty() {
        let versions = [];
        assert_eq!(
            ReleaseTracks::from_sorted_semver_iter(versions.into_iter()),
            ReleaseTracks(IndexMap::new())
        );
    }

    #[test]
    fn release_tracks_prerelease() {
        let versions = [version("1.0.0-beta.5")];
        assert_eq!(
            ReleaseTracks::from_sorted_semver_iter(versions.iter()),
            ReleaseTracks(IndexMap::new())
        );
    }

    #[test]
    fn release_tracks_multiple() {
        let versions = [
            "100.1.1",
            "100.1.0",
            "1.3.5",
            "1.2.5",
            "1.1.5",
            "0.4.0-rc.1",
            "0.3.23",
            "0.3.22",
            "0.3.21-pre.0",
            "0.3.20",
            "0.3.3",
            "0.3.2",
            "0.3.1",
            "0.3.0",
            "0.2.1",
            "0.2.0",
            "0.1.2",
            "0.1.1",
        ]
        .map(version);

        let release_tracks = ReleaseTracks::from_sorted_semver_iter(versions.iter());
        assert_eq!(
            release_tracks,
            ReleaseTracks(IndexMap::from([
                (
                    ReleaseTrackName::Major(100),
                    ReleaseTrackDetails {
                        highest: version("100.1.1")
                    }
                ),
                (
                    ReleaseTrackName::Major(1),
                    ReleaseTrackDetails {
                        highest: version("1.3.5")
                    }
                ),
                (
                    ReleaseTrackName::Minor(3),
                    ReleaseTrackDetails {
                        highest: version("0.3.23")
                    }
                ),
                (
                    ReleaseTrackName::Minor(2),
                    ReleaseTrackDetails {
                        highest: version("0.2.1")
                    }
                ),
                (
                    ReleaseTrackName::Minor(1),
                    ReleaseTrackDetails {
                        highest: version("0.1.2")
                    }
                ),
            ]))
        );

        let json = serde_json::from_str::<serde_json::Value>(
            &serde_json::to_string(&release_tracks).unwrap(),
        )
        .unwrap();
        assert_eq!(
            json,
            json!({
                "100": { "highest": "100.1.1" },
                "1": { "highest": "1.3.5" },
                "0.3": { "highest": "0.3.23" },
                "0.2": { "highest": "0.2.1" },
                "0.1": { "highest": "0.1.2" }
            })
        );
    }
}
