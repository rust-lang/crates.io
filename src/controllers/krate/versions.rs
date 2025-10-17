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
use crate::views::release_tracks::ReleaseTracks;
use axum::Json;
use axum::extract::FromRequestParts;
use axum_extra::extract::Query;
use crates_io_diesel_helpers::semver_ord;
use diesel::dsl::not;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use futures_util::{TryStreamExt, future};
use http::request::Parts;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
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

    let versions_and_publishers =
        list(crate_id, pagination.as_ref(), &params, &req, &mut conn).await?;

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

/// Seek-based pagination of versions
///
/// # Panics
///
/// This function will panic if `options` is built with `enable_pages` set to true.
async fn list(
    crate_id: i32,
    options: Option<&PaginationOptions>,
    params: &ListQueryParams,
    req: &Parts,
    conn: &mut AsyncPgConnection,
) -> AppResult<PaginatedVersionsAndPublishers> {
    use seek::*;

    let seek = match &params.sort.as_ref().map(|s| s.to_lowercase()).as_deref() {
        Some("date") => Seek::Date,
        _ => Seek::Semver,
    };

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
        assert!(
            !matches!(&options.page, Page::Numeric(_)),
            "?page= is not supported"
        );

        match seek.after(&options.page)? {
            Some(SeekPayload::Date(Date { created_at, id })) => {
                query = query.filter(
                    versions::created_at
                        .eq(created_at)
                        .and(versions::id.lt(id))
                        .or(versions::created_at.lt(created_at)),
                )
            }
            Some(SeekPayload::Semver(Semver { num, id })) => {
                query = query.filter(
                    versions::semver_ord
                        .eq(semver_ord(num.clone()))
                        .and(versions::id.lt(id))
                        .or(versions::semver_ord.lt(semver_ord(num))),
                )
            }
            None => {}
        }

        query = query.limit(options.per_page);
    }

    if seek == Seek::Date {
        query = query.order((versions::created_at.desc(), versions::id.desc()));
    } else {
        query = query.order((versions::semver_ord.desc(), versions::id.desc()));
    }

    let data: Vec<(Version, Option<User>)> = query.load(conn).await?;
    let mut next_page = None;
    if let Some(options) = options {
        next_page = next_seek_params(&data, options, |last| seek.to_payload(last))?
            .map(|p| req.query_with_params(p));
    };

    let release_tracks = if params.include()?.release_tracks {
        let mut sorted_versions = IndexSet::new();
        if options.is_some() {
            versions::table
                .filter(versions::crate_id.eq(crate_id))
                .filter(not(versions::yanked))
                .select(versions::num)
                .order(versions::semver_ord.desc())
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
            if seek == Seek::Date {
                sorted_versions.sort_unstable_by(|a, b| b.cmp(a));
            }
        }

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
                num: String,
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
                Seek::Semver => SeekPayload::Semver(Semver {
                    num: record.0.num.clone(),
                    id,
                }),
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
    if matches!(options.page, Page::Numeric(_)) || records.len() < options.per_page as usize {
        return Ok(None);
    }

    let mut opts = IndexMap::new();
    match options.page {
        Page::Unspecified | Page::Seek(_) => {
            let seek = f(records.last().unwrap());
            opts.insert("seek".into(), encode_seek(seek)?);
        }
        Page::Numeric(_) => unreachable!(),
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
