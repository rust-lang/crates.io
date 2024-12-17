//! Endpoint for searching and discovery functionality

use crate::auth::AuthCheck;
use axum::extract::FromRequestParts;
use axum_extra::extract::Query;
use axum_extra::json;
use axum_extra::response::ErasedJson;
use derive_more::Deref;
use diesel::dsl::{exists, sql, InnerJoinQuerySource, LeftJoinQuerySource};
use diesel::prelude::*;
use diesel::sql_types::{Bool, Text};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use diesel_full_text_search::*;
use http::request::Parts;
use tracing::Instrument;
use utoipa::IntoParams;

use crate::app::AppState;
use crate::controllers::helpers::Paginate;
use crate::models::{Crate, CrateOwner, OwnerKind, TopVersions, Version};
use crate::schema::*;
use crate::util::errors::{bad_request, AppResult};
use crate::views::EncodableCrate;

use crate::controllers::helpers::pagination::{Page, PaginationOptions};
use crate::models::krate::ALL_COLUMNS;
use crate::sql::{array_agg, canon_crate_name, lower};
use crate::util::string_excl_null::StringExclNull;
use crate::util::RequestUtils;

/// Returns a list of crates.
///
/// Called in a variety of scenarios in the front end, including:
/// - Alphabetical listing of crates
/// - List of crates under a specific owner
/// - Listing a user's followed crates
#[utoipa::path(
    get,
    path = "/api/v1/crates",
    params(ListQueryParams),
    tag = "crates",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn list_crates(
    app: AppState,
    params: ListQueryParams,
    req: Parts,
) -> AppResult<ErasedJson> {
    // Notes:
    // The different use cases this function covers is handled through passing
    // in parameters in the GET request.
    //
    // We would like to stop adding functionality in here. It was built like
    // this to keep the number of database queries low, though given Rust's
    // low performance overhead, this is a soft goal to have, and can afford
    // more database transactions if it aids understandability.
    //
    // All of the edge cases for this function are not currently covered
    // in testing, and if they fail, it is difficult to determine what
    // caused the break. In the future, we should look at splitting this
    // function out to cover the different use cases, and create unit tests
    // for them.

    let mut conn = app.db_read().await?;

    use diesel::sql_types::Float;
    use seek::*;

    let filter_params = FilterParams::from(params, &req, &mut conn).await?;
    let sort = filter_params.sort.as_deref();

    let selection = (
        ALL_COLUMNS,
        false.into_sql::<Bool>(),
        crate_downloads::downloads,
        recent_crate_downloads::downloads.nullable(),
        0_f32.into_sql::<Float>(),
        versions::num.nullable(),
        versions::yanked.nullable(),
    );

    let mut seek: Option<Seek> = None;
    let mut query = filter_params
        .make_query()?
        .inner_join(crate_downloads::table)
        .left_join(recent_crate_downloads::table)
        .left_join(default_versions::table)
        .left_join(versions::table.on(default_versions::version_id.eq(versions::id)))
        .select(selection);

    if let Some(q_string) = &filter_params.q_string {
        if !q_string.is_empty() {
            let q_string = q_string.as_str();

            let sort = sort.unwrap_or("relevance");

            query = query.order(Crate::with_name(q_string).desc());

            if sort == "relevance" {
                let q = sql::<TsQuery>("plainto_tsquery('english', ")
                    .bind::<Text, _>(q_string)
                    .sql(")");
                let rank = ts_rank_cd(crates::textsearchable_index_col, q);
                query = query.select((
                    ALL_COLUMNS,
                    Crate::with_name(q_string),
                    crate_downloads::downloads,
                    recent_crate_downloads::downloads.nullable(),
                    rank.clone(),
                    versions::num.nullable(),
                    versions::yanked.nullable(),
                ));
                seek = Some(Seek::Relevance);
                query = query.then_order_by(rank.desc())
            } else {
                query = query.select((
                    ALL_COLUMNS,
                    Crate::with_name(q_string),
                    crate_downloads::downloads,
                    recent_crate_downloads::downloads.nullable(),
                    0_f32.into_sql::<Float>(),
                    versions::num.nullable(),
                    versions::yanked.nullable(),
                ));
                seek = Some(Seek::Query);
            }
        }
    }

    // Any sort other than 'relevance' (default) would ignore exact crate name matches
    // Seek-based pagination requires a unique ordering to avoid unexpected row skipping
    // during pagination.
    // Therefore, when the ordering isn't unique an auxiliary ordering column should be added
    // to ensure predictable pagination behavior.
    if sort == Some("downloads") {
        seek = Some(Seek::Downloads);
        query = query.order((crate_downloads::downloads.desc(), crates::id.desc()))
    } else if sort == Some("recent-downloads") {
        seek = Some(Seek::RecentDownloads);
        query = query.order((
            recent_crate_downloads::downloads.desc().nulls_last(),
            crates::id.desc(),
        ))
    } else if sort == Some("recent-updates") {
        seek = Some(Seek::RecentUpdates);
        query = query.order((crates::updated_at.desc(), crates::id.desc()));
    } else if sort == Some("new") {
        seek = Some(Seek::New);
        query = query.order((crates::created_at.desc(), crates::id.desc()));
    } else {
        seek = seek.or(Some(Seek::Name));
        // Since the name is unique value, the inherent ordering becomes naturally unique.
        // Therefore, an additional auxiliary ordering column is unnecessary in this case.
        query = query.then_order_by(crates::name.asc())
    }

    let pagination: PaginationOptions = PaginationOptions::builder()
        .limit_page_numbers()
        .enable_seek(true)
        .gather(&req)?;

    let explicit_page = matches!(pagination.page, Page::Numeric(_));

    // To avoid breaking existing users, seek-based pagination is only used if an explicit page has
    // not been provided. This way clients relying on meta.next_page will use the faster seek-based
    // paginations, while client hardcoding pages handling will use the slower offset-based code.
    let (total, next_page, prev_page, data) = if !explicit_page && seek.is_some() {
        let seek = seek.unwrap();
        if let Some(condition) = seek
            .after(&pagination.page)?
            .map(|s| filter_params.seek_after(&s))
        {
            query = query.filter(condition);
        }

        // This does a full index-only scan over the crates table to gather how many crates were
        // published. Unfortunately on PostgreSQL counting the rows in a table requires scanning
        // the table, and the `total` field is part of the stable registries API.
        //
        // If this becomes a problem in the future the crates count could be denormalized, at least
        // for the filterless happy path.
        let count_query = filter_params.make_query()?.count();
        let query = query.pages_pagination_with_count_query(pagination, count_query);
        let span = info_span!("db.query", message = "SELECT ..., COUNT(*) FROM crates");
        let data = query.load::<Record>(&mut conn).instrument(span).await?;
        (
            data.total(),
            data.next_seek_params(|last| seek.to_payload(last))?
                .map(|p| req.query_with_params(p)),
            None,
            data.into_iter().collect::<Vec<_>>(),
        )
    } else {
        let count_query = filter_params.make_query()?.count();
        let query = query.pages_pagination_with_count_query(pagination, count_query);
        let span = info_span!("db.query", message = "SELECT ..., COUNT(*) FROM crates");
        let data = query.load::<Record>(&mut conn).instrument(span).await?;
        (
            data.total(),
            data.next_page_params().map(|p| req.query_with_params(p)),
            data.prev_page_params().map(|p| req.query_with_params(p)),
            data.into_iter().collect::<Vec<_>>(),
        )
    };

    let crates = data.iter().map(|(c, ..)| c).collect::<Vec<_>>();

    let span = info_span!("db.query", message = "SELECT ... FROM versions");
    let versions: Vec<Version> = Version::belonging_to(&crates)
        .filter(versions::yanked.eq(false))
        .select(Version::as_select())
        .load(&mut conn)
        .instrument(span)
        .await?;
    let versions = versions
        .grouped_by(&crates)
        .into_iter()
        .map(TopVersions::from_versions);

    let crates = versions
        .zip(data)
        .map(
            |(max_version, (krate, perfect_match, total, recent, _, default_version, yanked))| {
                EncodableCrate::from_minimal(
                    krate,
                    default_version.as_deref(),
                    yanked,
                    Some(&max_version),
                    perfect_match,
                    total,
                    Some(recent.unwrap_or(0)),
                )
            },
        )
        .collect::<Vec<_>>();

    Ok(json!({
        "crates": crates,
        "meta": {
            "total": total,
            "next_page": next_page,
            "prev_page": prev_page,
        },
    }))
}

#[derive(Debug, Deserialize, FromRequestParts, IntoParams)]
#[from_request(via(Query))]
#[into_params(parameter_in = Query)]
pub struct ListQueryParams {
    /// The sort order of the crates.
    ///
    /// Valid values: `alphabetical`, `relevance`, `downloads`,
    /// `recent-downloads`, `recent-updates`, `new`.
    ///
    /// Defaults to `relevance` if `q` is set, otherwise `alphabetical`.
    sort: Option<String>,

    /// A search query string.
    #[serde(rename = "q")]
    #[param(inline)]
    q_string: Option<StringExclNull>,

    /// Set to `yes` to include yanked crates.
    #[param(example = "yes")]
    include_yanked: Option<String>,

    /// If set, only return crates that belong to this category, or one
    /// of its subcategories.
    #[param(inline)]
    category: Option<StringExclNull>,

    /// If set, only return crates matching all the given keywords.
    ///
    /// This parameter expects a space-separated list of keywords.
    #[param(inline)]
    all_keywords: Option<StringExclNull>,

    /// If set, only return crates matching the given keyword
    /// (ignored if `all_keywords` is set).
    #[param(inline)]
    keyword: Option<StringExclNull>,

    /// If set, only return crates with names that start with the given letter
    /// (ignored if `all_keywords` or `keyword` are set).
    #[param(inline)]
    letter: Option<StringExclNull>,

    /// If set, only crates owned by the given crates.io user ID are returned
    /// (ignored if `all_keywords`, `keyword`, or `letter` are set).
    user_id: Option<i32>,

    /// If set, only crates owned by the given crates.io team ID are returned
    /// (ignored if `all_keywords`, `keyword`, `letter`, or `user_id` are set).
    team_id: Option<i32>,

    /// If set, only crates owned by users the current user follows are returned
    /// (ignored if `all_keywords`, `keyword`, `letter`, `user_id`,
    /// or `team_id` are set).
    ///
    /// The exact value of this parameter is ignored, but it must not be empty.
    #[param(example = "yes")]
    following: Option<String>,

    /// If set, only crates with the specified names are returned (ignored
    /// if `all_keywords`, `keyword`, `letter`, `user_id`, `team_id`,
    /// or `following` are set).
    #[serde(rename = "ids[]", default)]
    #[param(inline)]
    ids: Vec<StringExclNull>,
}

impl ListQueryParams {
    pub fn include_yanked(&self) -> bool {
        let include_yanked = self.include_yanked.as_ref();
        include_yanked.map(|s| s == "yes").unwrap_or(true)
    }
}

#[derive(Deref)]
struct FilterParams {
    #[deref]
    search_params: ListQueryParams,
    auth_user_id: Option<i32>,
}

impl FilterParams {
    async fn from(
        search_params: ListQueryParams,
        parts: &Parts,
        conn: &mut AsyncPgConnection,
    ) -> AppResult<Self> {
        let auth_user_id = match search_params.following {
            Some(_) => Some(AuthCheck::default().check(parts, conn).await?.user_id()),
            None => None,
        };

        Ok(Self {
            search_params,
            auth_user_id,
        })
    }
}

impl FilterParams {
    fn make_query(&self) -> AppResult<crates::BoxedQuery<'_, diesel::pg::Pg>> {
        let mut query = crates::table.into_boxed();

        if let Some(q_string) = &self.q_string {
            if !q_string.is_empty() {
                let q = sql::<TsQuery>("plainto_tsquery('english', ")
                    .bind::<Text, _>(q_string.as_str())
                    .sql(")");
                query = query.filter(
                    q.matches(crates::textsearchable_index_col)
                        .or(Crate::loosly_matches_name(q_string.as_str())),
                );
            }
        }

        if let Some(cat) = &self.category {
            query = query.filter(
                crates::id.eq_any(
                    crates_categories::table
                        .select(crates_categories::crate_id)
                        .inner_join(categories::table)
                        .filter(
                            categories::slug
                                .eq(cat.as_str())
                                .or(categories::slug.like(format!("{cat}::%"))),
                        ),
                ),
            );
        }

        if let Some(kws) = &self.all_keywords {
            let names: Vec<_> = kws
                .split_whitespace()
                .map(|name| name.to_lowercase())
                .collect();

            query = query.filter(
                crates_keywords::table
                    .inner_join(keywords::table)
                    .filter(crates_keywords::crate_id.eq(crates::id))
                    .select(array_agg(keywords::keyword))
                    .single_value()
                    .contains(names),
            );
        } else if let Some(kw) = &self.keyword {
            query = query.filter(
                crates::id.eq_any(
                    crates_keywords::table
                        .select(crates_keywords::crate_id)
                        .inner_join(keywords::table)
                        .filter(lower(keywords::keyword).eq(lower(kw.as_str()))),
                ),
            );
        } else if let Some(letter) = &self.letter {
            let pattern = format!(
                "{}%",
                letter
                    .chars()
                    .next()
                    .ok_or_else(|| bad_request("letter value must contain 1 character"))?
                    .to_lowercase()
                    .collect::<String>()
            );
            query = query.filter(canon_crate_name(crates::name).like(pattern));
        } else if let Some(user_id) = self.user_id {
            query = query.filter(
                crates::id.eq_any(
                    CrateOwner::by_owner_kind(OwnerKind::User)
                        .select(crate_owners::crate_id)
                        .filter(crate_owners::owner_id.eq(user_id)),
                ),
            );
        } else if let Some(team_id) = self.team_id {
            query = query.filter(
                crates::id.eq_any(
                    CrateOwner::by_owner_kind(OwnerKind::Team)
                        .select(crate_owners::crate_id)
                        .filter(crate_owners::owner_id.eq(team_id)),
                ),
            );
        } else if let Some(user_id) = self.auth_user_id {
            query = query.filter(
                crates::id.eq_any(
                    follows::table
                        .select(follows::crate_id)
                        .filter(follows::user_id.eq(user_id)),
                ),
            );
        } else if !self.ids.is_empty() {
            query = query.filter(crates::name.eq_any(self.ids.iter().map(|s| s.as_str())));
        }

        if !self.include_yanked() {
            query = query.filter(exists(
                versions::table
                    .filter(versions::crate_id.eq(crates::id))
                    .filter(versions::yanked.eq(false)),
            ));
        }

        Ok(query)
    }

    fn seek_after(&self, seek_payload: &seek::SeekPayload) -> BoxedCondition<'_> {
        use seek::*;

        let crates_aliased = alias!(crates as crates_aliased);
        let crate_name_by_id = |id: i32| {
            crates_aliased
                .find(id)
                .select(crates_aliased.field(crates::name))
                .single_value()
        };
        let conditions: Vec<BoxedCondition<'_>> = match *seek_payload {
            SeekPayload::Name(Name { id }) => {
                // Equivalent of:
                // `WHERE name > name'`
                vec![Box::new(crates::name.nullable().gt(crate_name_by_id(id)))]
            }
            SeekPayload::New(New { created_at, id }) => {
                // Equivalent of:
                // `WHERE (created_at = created_at' AND id < id') OR created_at < created_at'`
                vec![
                    Box::new(
                        crates::created_at
                            .eq(created_at)
                            .and(crates::id.lt(id))
                            .nullable(),
                    ),
                    Box::new(crates::created_at.lt(created_at).nullable()),
                ]
            }
            SeekPayload::RecentUpdates(RecentUpdates { updated_at, id }) => {
                // Equivalent of:
                // `WHERE (updated_at = updated_at' AND id < id') OR updated_at < updated_at'`
                vec![
                    Box::new(
                        crates::updated_at
                            .eq(updated_at)
                            .and(crates::id.lt(id))
                            .nullable(),
                    ),
                    Box::new(crates::updated_at.lt(updated_at).nullable()),
                ]
            }
            SeekPayload::RecentDownloads(RecentDownloads {
                recent_downloads,
                id,
            }) => {
                // Equivalent of:
                // for recent_downloads is not None:
                // `WHERE (recent_downloads = recent_downloads' AND id < id')
                //      OR (recent_downloads < recent_downloads' OR recent_downloads IS NULL)`
                // for recent_downloads is None:
                // `WHERE (recent_downloads IS NULL AND id < id')`
                match recent_downloads {
                    Some(dl) => {
                        vec![
                            Box::new(
                                recent_crate_downloads::downloads
                                    .eq(dl)
                                    .and(crates::id.lt(id))
                                    .nullable(),
                            ),
                            Box::new(
                                recent_crate_downloads::downloads
                                    .lt(dl)
                                    .or(recent_crate_downloads::downloads.is_null())
                                    .nullable(),
                            ),
                        ]
                    }
                    None => {
                        vec![Box::new(
                            recent_crate_downloads::downloads
                                .is_null()
                                .and(crates::id.lt(id))
                                .nullable(),
                        )]
                    }
                }
            }
            SeekPayload::Downloads(Downloads { downloads, id }) => {
                // Equivalent of:
                // `WHERE (downloads = downloads' AND id < id') OR downloads < downloads'`
                vec![
                    Box::new(
                        crate_downloads::downloads
                            .eq(downloads)
                            .and(crates::id.lt(id))
                            .nullable(),
                    ),
                    Box::new(crate_downloads::downloads.lt(downloads).nullable()),
                ]
            }
            SeekPayload::Query(Query { exact_match, id }) => {
                // Equivalent of:
                // `WHERE (exact_match = exact_match' AND name < name') OR exact_match <
                // exact_match'`
                let q_string = self.q_string.as_ref().expect("q_string should not be None");
                let name_exact_match = Crate::with_name(q_string);
                vec![
                    Box::new(
                        name_exact_match
                            .eq(exact_match)
                            .and(crates::name.nullable().gt(crate_name_by_id(id)))
                            .nullable(),
                    ),
                    Box::new(name_exact_match.lt(exact_match).nullable()),
                ]
            }
            SeekPayload::Relevance(Relevance {
                exact_match: exact,
                rank: rank_in,
                id,
            }) => {
                // Equivalent of:
                // `WHERE (exact_match = exact_match' AND rank = rank' AND name > name')
                //      OR (exact_match = exact_match' AND rank < rank')
                //      OR exact_match < exact_match'`
                let q_string = self.q_string.as_ref().expect("q_string should not be None");
                let q = sql::<TsQuery>("plainto_tsquery('english', ")
                    .bind::<Text, _>(q_string.as_str())
                    .sql(")");
                let rank = ts_rank_cd(crates::textsearchable_index_col, q);
                let name_exact_match = Crate::with_name(q_string.as_str());
                vec![
                    Box::new(
                        name_exact_match
                            .eq(exact)
                            .and(rank.clone().eq(rank_in))
                            .and(crates::name.nullable().gt(crate_name_by_id(id)))
                            .nullable(),
                    ),
                    Box::new(name_exact_match.eq(exact).and(rank.lt(rank_in)).nullable()),
                    Box::new(name_exact_match.lt(exact).nullable()),
                ]
            }
        };

        conditions
            .into_iter()
            .fold(
                None,
                |merged_condition: Option<BoxedCondition<'_>>, condition| {
                    Some(match merged_condition {
                        Some(merged) => Box::new(merged.or(condition)),
                        None => condition,
                    })
                },
            )
            .expect("should be a reduced BoxedCondition")
    }
}

mod seek {
    use super::Record;
    use crate::controllers::helpers::pagination::seek;
    use crate::models::Crate;
    use chrono::naive::serde::ts_microseconds;

    seek!(
        pub enum Seek {
            Name {
                id: i32,
            },
            New {
                #[serde(with = "ts_microseconds")]
                created_at: chrono::NaiveDateTime,
                id: i32,
            },
            RecentUpdates {
                #[serde(with = "ts_microseconds")]
                updated_at: chrono::NaiveDateTime,
                id: i32,
            },
            RecentDownloads {
                recent_downloads: Option<i64>,
                id: i32,
            },
            Downloads {
                downloads: i64,
                id: i32,
            },
            Query {
                exact_match: bool,
                id: i32,
            },
            Relevance {
                exact_match: bool,
                rank: f32,
                id: i32,
            },
        }
    );

    impl Seek {
        pub(crate) fn to_payload(&self, record: &Record) -> SeekPayload {
            let (
                Crate {
                    id,
                    updated_at,
                    created_at,
                    ..
                },
                exact_match,
                downloads,
                recent_downloads,
                rank,
                ..,
            ) = *record;

            match *self {
                Seek::Name => SeekPayload::Name(Name { id }),
                Seek::New => SeekPayload::New(New { created_at, id }),
                Seek::RecentUpdates => SeekPayload::RecentUpdates(RecentUpdates { updated_at, id }),
                Seek::RecentDownloads => SeekPayload::RecentDownloads(RecentDownloads {
                    recent_downloads,
                    id,
                }),
                Seek::Downloads => SeekPayload::Downloads(Downloads { downloads, id }),
                Seek::Query => SeekPayload::Query(Query { exact_match, id }),
                Seek::Relevance => SeekPayload::Relevance(Relevance {
                    exact_match,
                    rank,
                    id,
                }),
            }
        }
    }
}

type Record = (
    Crate,
    bool,
    i64,
    Option<i64>,
    f32,
    Option<String>,
    Option<bool>,
);

type QuerySource = LeftJoinQuerySource<
    LeftJoinQuerySource<
        LeftJoinQuerySource<
            InnerJoinQuerySource<crates::table, crate_downloads::table>,
            recent_crate_downloads::table,
        >,
        default_versions::table,
    >,
    versions::table,
    diesel::dsl::Eq<default_versions::version_id, versions::id>,
>;

type BoxedCondition<'a> = Box<
    dyn BoxableExpression<QuerySource, diesel::pg::Pg, SqlType = diesel::sql_types::Nullable<Bool>>
        + 'a,
>;
