//! Endpoint for versions of a crate

use axum::extract::Path;
use axum::Json;
use diesel::connection::DefaultLoadingMode;
use diesel::prelude::*;
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use http::request::Parts;
use indexmap::IndexMap;
use serde_json::Value;
use std::cmp::Reverse;

use crate::app::AppState;
use crate::controllers::helpers::pagination::{encode_seek, Page, PaginationOptions};
use crate::models::{Crate, User, Version, VersionOwnerAction};
use crate::schema::{crates, users, versions};
use crate::tasks::spawn_blocking;
use crate::util::diesel::Conn;
use crate::util::errors::{crate_not_found, AppResult};
use crate::util::RequestUtils;
use crate::views::EncodableVersion;

/// Handles the `GET /crates/:crate_id/versions` route.
pub async fn versions(
    state: AppState,
    Path(crate_name): Path<String>,
    req: Parts,
) -> AppResult<Json<Value>> {
    let conn = state.db_read().await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        let crate_id: i32 = Crate::by_name(&crate_name)
            .select(crates::id)
            .first(conn)
            .optional()?
            .ok_or_else(|| crate_not_found(&crate_name))?;

        let mut pagination = None;
        let params = req.query();
        // To keep backward compatibility, we paginate only if per_page is provided
        if params.get("per_page").is_some() {
            pagination = Some(
                PaginationOptions::builder()
                    .enable_seek(true)
                    .enable_pages(true)
                    .gather(&req)?,
            );
        }

        // Sort by semver by default
        let versions_and_publishers = match params.get("sort").map(|s| s.to_lowercase()).as_deref()
        {
            Some("date") => list_by_date(crate_id, pagination.as_ref(), &req, conn)?,
            _ => list_by_semver(crate_id, pagination.as_ref(), &req, conn)?,
        };

        let versions = versions_and_publishers
            .data
            .iter()
            .map(|(v, _)| v)
            .cloned()
            .collect::<Vec<_>>();
        let versions = versions_and_publishers
            .data
            .into_iter()
            .zip(VersionOwnerAction::for_versions(conn, &versions)?)
            .map(|((v, pb), aas)| EncodableVersion::from(v, &crate_name, pb, aas))
            .collect::<Vec<_>>();

        Ok(Json(match pagination {
            Some(_) => json!({ "versions": versions, "meta": versions_and_publishers.meta }),
            None => json!({ "versions": versions }),
        }))
    })
    .await
}

/// Seek-based pagination of versions by date
///
/// # Panics
///
/// This function will panic if `option` is built with `enable_pages` set to true.
fn list_by_date(
    crate_id: i32,
    options: Option<&PaginationOptions>,
    req: &Parts,
    conn: &mut impl Conn,
) -> AppResult<PaginatedVersionsAndPublishers> {
    use seek::*;

    let mut query = versions::table
        .filter(versions::crate_id.eq(crate_id))
        .left_outer_join(users::table)
        .select((versions::all_columns, users::all_columns.nullable()))
        .into_boxed();

    if let Some(options) = options {
        if let Page::Numeric(page) = options.page {
            query = query.offset((page.saturating_sub(1) as i64).saturating_mul(options.per_page));
        } else if let Some(SeekPayload::Date(Date { created_at, id })) =
            Seek::Date.after(&options.page)?
        {
            query = query.filter(
                versions::created_at
                    .eq(created_at)
                    .and(versions::id.lt(id))
                    .or(versions::created_at.lt(created_at)),
            );
        }
        query = query.limit(options.per_page);
    }

    query = query.order((versions::created_at.desc(), versions::id.desc()));

    let data: Vec<(Version, Option<User>)> = query.load(conn)?;
    let (next_page, prev_page) = match options.map(|opt| &opt.page) {
        Some(Page::Numeric(_)) => (
            next_page_params(&data, options.expect("options")).map(|p| req.query_with_params(p)),
            prev_page_params(options.expect("options")).map(|p| req.query_with_params(p)),
        ),
        Some(Page::Unspecified) | Some(Page::Seek(_)) => (
            next_seek_params(&data, options.expect("options"), |last| {
                Seek::Date.to_payload(last)
            })?
            .map(|p| req.query_with_params(p)),
            None,
        ),
        None => (None, None),
    };

    // Since the total count is retrieved through an additional query, to maintain consistency
    // with other pagination methods, we only make a count query while data is not empty.
    let total = if !data.is_empty() {
        versions::table
            .filter(versions::crate_id.eq(crate_id))
            .count()
            .get_result(conn)?
    } else {
        0
    };

    Ok(PaginatedVersionsAndPublishers {
        data,
        meta: ResponseMeta {
            total,
            next_page,
            prev_page,
        },
    })
}

/// Seek-based pagination of versions by semver
///
/// # Panics
///
/// This function will panic if `option` is built with `enable_pages` set to true.

// Unfortunately, Heroku Postgres has no support for the semver PG extension.
// Therefore, we need to perform both sorting and pagination manually on the server.
fn list_by_semver(
    crate_id: i32,
    options: Option<&PaginationOptions>,
    req: &Parts,
    conn: &mut impl Conn,
) -> AppResult<PaginatedVersionsAndPublishers> {
    use seek::*;

    let (data, total) = if let Some(options) = options {
        // Since versions will only increase in the future and both sorting and pagination need to
        // happen on the app server, implementing it with fetching only the data needed for sorting
        // and pagination, then making another query for the data to respond with, would minimize
        // payload and memory usage. This way, we can utilize the sorted map and enrich it later
        // without sorting twice.
        // Sorting by semver but opted for id as the seek key because num can be quite lengthy,
        // while id values are significantly smaller.
        let mut sorted_versions = IndexMap::new();
        for result in versions::table
            .filter(versions::crate_id.eq(crate_id))
            .select((versions::id, versions::num))
            .load_iter::<(i32, String), DefaultLoadingMode>(conn)?
        {
            let (id, num) = result?;
            sorted_versions.insert(id, (num, None));
        }
        sorted_versions.sort_by_cached_key(|_, (num, _)| Reverse(semver::Version::parse(num).ok()));

        let idx = if let Page::Numeric(page) = options.page {
            Some((page.saturating_sub(1) as usize).saturating_mul(options.per_page as usize))
        } else if let Some(SeekPayload::Semver(Semver { id })) =
            Seek::Semver.after(&options.page)?
        {
            sorted_versions
                .get_index_of(&id)
                .filter(|i| i + 1 < sorted_versions.len())
                .map(|i| i + 1)
        } else {
            Some(0)
        };

        // Only make query while end > start
        if let Some((start, end)) = idx
            .map(|start| {
                let end = (start + options.per_page as usize).min(sorted_versions.len());
                (start, end)
            })
            .filter(|(start, end)| end > start)
        {
            let ids = sorted_versions[start..end]
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            for result in versions::table
                .filter(versions::crate_id.eq(crate_id))
                .left_outer_join(users::table)
                .select((versions::all_columns, users::all_columns.nullable()))
                .filter(versions::id.eq_any(ids))
                .load_iter::<(Version, Option<User>), DefaultLoadingMode>(conn)?
            {
                let row = result?;
                sorted_versions.insert(row.0.id, (row.0.num.to_owned(), Some(row)));
            }

            let len = sorted_versions.len();
            (
                sorted_versions
                    .into_values()
                    .filter_map(|(_, v)| v)
                    .collect(),
                len,
            )
        } else {
            (vec![], 0)
        }
    } else {
        let mut data: Vec<(Version, Option<User>)> = versions::table
            .filter(versions::crate_id.eq(crate_id))
            .left_outer_join(users::table)
            .select((versions::all_columns, users::all_columns.nullable()))
            .load(conn)?;
        data.sort_by_cached_key(|(version, _)| Reverse(semver::Version::parse(&version.num).ok()));
        let total = data.len();
        (data, total)
    };

    let (next_page, prev_page) = match options.map(|opt| &opt.page) {
        Some(Page::Numeric(_)) => (
            next_page_params(&data, options.expect("options")).map(|p| req.query_with_params(p)),
            prev_page_params(options.expect("options")).map(|p| req.query_with_params(p)),
        ),
        Some(Page::Unspecified) | Some(Page::Seek(_)) => (
            next_seek_params(&data, options.expect("options"), |last| {
                Seek::Semver.to_payload(last)
            })?
            .map(|p| req.query_with_params(p)),
            None,
        ),
        None => (None, None),
    };

    Ok(PaginatedVersionsAndPublishers {
        data,
        meta: ResponseMeta {
            total: total as i64,
            next_page,
            prev_page,
        },
    })
}

mod seek {
    use crate::controllers::helpers::pagination::seek;
    use crate::models::{User, Version};
    use chrono::naive::serde::ts_microseconds;

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
                created_at: chrono::NaiveDateTime,
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

fn next_page_params<T>(
    records: &[T],
    options: &PaginationOptions,
) -> Option<IndexMap<String, String>> {
    if records.len() < options.per_page as usize {
        return None;
    }

    let mut opts = IndexMap::new();
    match options.page {
        Page::Numeric(n) => opts.insert("page".into(), (n + 1).to_string()),
        Page::Unspecified | Page::Seek(_) => return None,
    };
    Some(opts)
}

fn prev_page_params(options: &PaginationOptions) -> Option<IndexMap<String, String>> {
    let mut opts = IndexMap::new();
    match options.page {
        Page::Numeric(1) | Page::Unspecified | Page::Seek(_) => return None,
        Page::Numeric(n) => opts.insert("page".into(), (n - 1).to_string()),
    };
    Some(opts)
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

#[derive(Serialize)]
struct ResponseMeta {
    total: i64,
    next_page: Option<String>,
    prev_page: Option<String>,
}
