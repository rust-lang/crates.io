//! Endpoint for searching and discovery functionality

use crate::auth::AuthCheck;
use diesel::dsl::*;
use diesel::sql_types::Array;
use diesel_full_text_search::*;
use indexmap::IndexMap;
use once_cell::sync::OnceCell;

use crate::controllers::cargo_prelude::*;
use crate::controllers::helpers::Paginate;
use crate::models::{Crate, CrateOwner, CrateVersions, OwnerKind, TopVersions, Version};
use crate::schema::*;
use crate::util::errors::bad_request;
use crate::views::EncodableCrate;

use crate::controllers::helpers::pagination::{Page, Paginated, PaginationOptions};
use crate::models::krate::ALL_COLUMNS;
use crate::sql::{array_agg, canon_crate_name, lower};

/// Handles the `GET /crates` route.
/// Returns a list of crates. Called in a variety of scenarios in the
/// front end, including:
/// - Alphabetical listing of crates
/// - List of crates under a specific owner
/// - Listing a user's followed crates
///
/// Notes:
/// The different use cases this function covers is handled through passing
/// in parameters in the GET request.
///
/// We would like to stop adding functionality in here. It was built like
/// this to keep the number of database queries low, though given Rust's
/// low performance overhead, this is a soft goal to have, and can afford
/// more database transactions if it aids understandability.
///
/// All of the edge cases for this function are not currently covered
/// in testing, and if they fail, it is difficult to determine what
/// caused the break. In the future, we should look at splitting this
/// function out to cover the different use cases, and create unit tests
/// for them.
pub async fn search(app: AppState, req: Parts) -> AppResult<Json<Value>> {
    spawn_blocking(move || {
        use diesel::sql_types::Bool;

        let params = req.query();
        let option_param = |s| params.get(s).map(|v| v.as_str());
        let sort = option_param("sort");
        let include_yanked = option_param("include_yanked")
            .map(|s| s == "yes")
            .unwrap_or(true);

        // Remove 0x00 characters from the query string because Postgres can not
        // handle them and will return an error, which would cause us to throw
        // an Internal Server Error ourselves.
        let q_string = option_param("q").map(|q| q.replace('\u{0}', ""));

        let filter_params = FilterParams {
            q_string: q_string.as_deref(),
            include_yanked,
            category: option_param("category"),
            all_keywords: option_param("all_keywords"),
            keyword: option_param("keyword"),
            letter: option_param("letter"),
            user_id: option_param("user_id").and_then(|s| s.parse::<i32>().ok()),
            team_id: option_param("team_id").and_then(|s| s.parse::<i32>().ok()),
            following: option_param("following").is_some(),
            has_ids: option_param("ids[]").is_some(),
            ..Default::default()
        };

        let selection = (
            ALL_COLUMNS,
            false.into_sql::<Bool>(),
            recent_crate_downloads::downloads.nullable(),
        );

        let conn = &mut *app.db_read()?;
        let mut supports_seek = filter_params.supports_seek();
        let mut query = filter_params
            .make_query(&req, conn)?
            .left_join(recent_crate_downloads::table)
            .select(selection);

        if let Some(q_string) = &q_string {
            // Searching with a query string always puts the exact match at the start of the results,
            // so we can't support seek-based pagination with it.
            supports_seek = false;

            if !q_string.is_empty() {
                let sort = sort.unwrap_or("relevance");

                query = query.select((
                    ALL_COLUMNS,
                    Crate::with_name(q_string),
                    recent_crate_downloads::downloads.nullable(),
                ));
                query = query.order(Crate::with_name(q_string).desc());

                if sort == "relevance" {
                    let q = to_tsquery_with_search_config(
                        configuration::TsConfigurationByName("english"),
                        q_string,
                    );
                    let rank = ts_rank_cd(crates::textsearchable_index_col, q);
                    query = query.then_order_by(rank.desc())
                }
            }
        }

        // Any sort other than 'relevance' (default) would ignore exact crate name matches
        if sort == Some("downloads") {
            // Custom sorting is not supported yet with seek.
            supports_seek = false;

            query = query.order(crates::downloads.desc())
        } else if sort == Some("recent-downloads") {
            // Custom sorting is not supported yet with seek.
            supports_seek = false;

            query = query.order(recent_crate_downloads::downloads.desc().nulls_last())
        } else if sort == Some("recent-updates") {
            // Custom sorting is not supported yet with seek.
            supports_seek = false;

            query = query.order(crates::updated_at.desc());
        } else if sort == Some("new") {
            // Custom sorting is not supported yet with seek.
            supports_seek = false;

            query = query.order(crates::created_at.desc());
        } else {
            query = query.then_order_by(crates::name.asc())
        }

        let pagination: PaginationOptions = PaginationOptions::builder()
            .limit_page_numbers()
            .enable_seek(supports_seek)
            .gather(&req)?;

        let (explicit_page, seek) = match pagination.page {
            Page::Numeric(_) => (true, None),
            Page::Seek(ref s) => (false, Some(s.decode::<i32>()?)),
            Page::Unspecified => (false, None),
        };

        // To avoid breaking existing users, seek-based pagination is only used if an explicit page has
        // not been provided. This way clients relying on meta.next_page will use the faster seek-based
        // paginations, while client hardcoding pages handling will use the slower offset-based code.
        let (total, next_page, prev_page, data, conn) = if supports_seek && !explicit_page {
            // Equivalent of:
            // `WHERE name > (SELECT name FROM crates WHERE id = $1) LIMIT $2`
            query = query.limit(pagination.per_page);
            if let Some(seek) = seek {
                let crate_name: String = crates::table
                    .find(seek)
                    .select(crates::name)
                    .get_result(conn)?;
                query = query.filter(crates::name.gt(crate_name));
            }

            // This does a full index-only scan over the crates table to gather how many crates were
            // published. Unfortunately on PostgreSQL counting the rows in a table requires scanning
            // the table, and the `total` field is part of the stable registries API.
            //
            // If this becomes a problem in the future the crates count could be denormalized, at least
            // for the filterless happy path.
            let count_query = filter_params.make_query(&req, conn)?.count();
            let total: i64 = info_span!("db.query", message = "SELECT COUNT(*) FROM crates")
                .in_scope(|| count_query.get_result(conn))?;

            let results: Vec<(Crate, bool, Option<i64>)> =
                info_span!("db.query", message = "SELECT ... FROM crates")
                    .in_scope(|| query.load(conn))?;

            let next_page = if let Some(last) = results.last() {
                let mut params = IndexMap::new();
                params.insert(
                    "seek".into(),
                    crate::controllers::helpers::pagination::encode_seek(last.0.id)?,
                );
                Some(req.query_with_params(params))
            } else {
                None
            };

            (total, next_page, None, results, conn)
        } else {
            let query = query.pages_pagination_with_count_query(
                pagination,
                filter_params.make_query(&req, conn)?.count(),
            );
            let data: Paginated<(Crate, bool, Option<i64>)> =
                info_span!("db.query", message = "SELECT ..., COUNT(*) FROM crates")
                    .in_scope(|| query.load(conn))?;
            (
                data.total(),
                data.next_page_params().map(|p| req.query_with_params(p)),
                data.prev_page_params().map(|p| req.query_with_params(p)),
                data.into_iter().collect::<Vec<_>>(),
                conn,
            )
        };

        let perfect_matches = data.iter().map(|&(_, b, _)| b).collect::<Vec<_>>();
        let recent_downloads = data
            .iter()
            .map(|&(_, _, s)| s.unwrap_or(0))
            .collect::<Vec<_>>();
        let crates = data.into_iter().map(|(c, _, _)| c).collect::<Vec<_>>();

        let versions: Vec<Version> = info_span!("db.query", message = "SELECT ... FROM versions")
            .in_scope(|| crates.versions().load(conn))?;
        let versions = versions
            .grouped_by(&crates)
            .into_iter()
            .map(TopVersions::from_versions);

        let crates = versions
            .zip(crates)
            .zip(perfect_matches)
            .zip(recent_downloads)
            .map(
                |(((max_version, krate), perfect_match), recent_downloads)| {
                    EncodableCrate::from_minimal(
                        krate,
                        Some(&max_version),
                        Some(vec![]),
                        perfect_match,
                        Some(recent_downloads),
                    )
                },
            )
            .collect::<Vec<_>>();

        Ok(Json(json!({
            "crates": crates,
            "meta": {
                "total": total,
                "next_page": next_page,
                "prev_page": prev_page,
            },
        })))
    })
    .await
}

#[derive(Default)]
struct FilterParams<'a> {
    q_string: Option<&'a str>,
    include_yanked: bool,
    category: Option<&'a str>,
    all_keywords: Option<&'a str>,
    keyword: Option<&'a str>,
    letter: Option<&'a str>,
    user_id: Option<i32>,
    team_id: Option<i32>,
    following: bool,
    has_ids: bool,
    _auth_user_id: OnceCell<i32>,
    _ids: OnceCell<Option<Vec<String>>>,
}

impl<'a> FilterParams<'a> {
    fn ids(&self, req: &Parts) -> Option<&[String]> {
        self._ids
            .get_or_init(|| {
                if self.has_ids {
                    let query_bytes = req.uri.query().unwrap_or("").as_bytes();
                    let v = url::form_urlencoded::parse(query_bytes)
                        .filter(|(key, _)| key == "ids[]")
                        .map(|(_, value)| value.to_string())
                        .collect::<Vec<_>>();
                    Some(v)
                } else {
                    None
                }
            })
            .as_deref()
    }

    fn authed_user_id(&self, req: &Parts, conn: &mut PgConnection) -> AppResult<&i32> {
        self._auth_user_id.get_or_try_init(|| {
            let user_id = AuthCheck::default().check(req, conn)?.user_id();
            Ok(user_id)
        })
    }

    fn supports_seek(&self) -> bool {
        // Calculating the total number of results with filters is supported but paging is not supported yet.
        !(self.q_string.is_some()
            || self.category.is_some()
            || self.all_keywords.is_some()
            || self.keyword.is_some()
            || self.letter.is_some()
            || self.user_id.is_some()
            || self.team_id.is_some()
            || self.following
            || self.has_ids
            || !self.include_yanked)
    }

    fn make_query(
        &'a self,
        req: &Parts,
        conn: &mut PgConnection,
    ) -> AppResult<crates::BoxedQuery<'a, diesel::pg::Pg>> {
        use diesel::sql_types::Text;
        let mut query = crates::table.into_boxed();

        if let Some(q_string) = self.q_string {
            if !q_string.is_empty() {
                let q = to_tsquery_with_search_config(
                    configuration::TsConfigurationByName("english"),
                    q_string,
                );
                query = query.filter(
                    q.matches(crates::textsearchable_index_col)
                        .or(Crate::loosly_matches_name(q_string)),
                );
            }
        }

        if let Some(cat) = self.category {
            query = query.filter(
                crates::id.eq_any(
                    crates_categories::table
                        .select(crates_categories::crate_id)
                        .inner_join(categories::table)
                        .filter(
                            categories::slug
                                .eq(cat)
                                .or(categories::slug.like(format!("{cat}::%"))),
                        ),
                ),
            );
        }

        if let Some(kws) = self.all_keywords {
            let names: Vec<_> = kws
                .split_whitespace()
                .map(|name| name.to_lowercase())
                .collect();

            query = query.filter(
                // FIXME: Just use `.contains` in Diesel 2.0
                // https://github.com/diesel-rs/diesel/issues/2066
                Contains::new(
                    crates_keywords::table
                        .inner_join(keywords::table)
                        .filter(crates_keywords::crate_id.eq(crates::id))
                        .select(array_agg(keywords::keyword))
                        .single_value(),
                    names.into_sql::<Array<Text>>(),
                ),
            );
        } else if let Some(kw) = self.keyword {
            query = query.filter(
                crates::id.eq_any(
                    crates_keywords::table
                        .select(crates_keywords::crate_id)
                        .inner_join(keywords::table)
                        .filter(lower(keywords::keyword).eq(lower(kw))),
                ),
            );
        } else if let Some(letter) = self.letter {
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
        } else if self.following {
            let user_id = self.authed_user_id(req, conn)?;
            query = query.filter(
                crates::id.eq_any(
                    follows::table
                        .select(follows::crate_id)
                        .filter(follows::user_id.eq(user_id)),
                ),
            );
        } else if self.ids(req).is_some() {
            query = query.filter(crates::name.eq_any(self.ids(req).unwrap()));
        }

        if !self.include_yanked {
            query = query.filter(exists(
                versions::table
                    .filter(versions::crate_id.eq(crates::id))
                    .filter(versions::yanked.eq(false)),
            ));
        }

        Ok(query)
    }
}

diesel::infix_operator!(Contains, "@>");
