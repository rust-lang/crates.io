//! Endpoint for searching and discovery functionality

use diesel::dsl::*;
use diesel_full_text_search::*;
use indexmap::IndexMap;
use serde::Deserialize;

use crate::controllers::cargo_prelude::*;
use crate::controllers::helpers::Paginate;
use crate::controllers::util::AuthenticatedUser;
use crate::models::{Crate, CrateBadge, CrateOwner, CrateVersions, OwnerKind, Version};
use crate::schema::*;
use crate::util::errors::{bad_request, ChainError};
use crate::views::EncodableCrate;

use crate::controllers::helpers::pagination::Paginated;
use crate::models::krate::{canon_crate_name, ALL_COLUMNS};

#[derive(Deserialize, Debug, Clone, Copy)]
struct QueryParams<'a> {
    all_keywords: Option<&'a str>,
    category: Option<&'a str>,
    following: Option<&'a str>,
    include_yanked: Option<&'a str>,
    keyword: Option<&'a str>,
    letter: Option<&'a str>,
    q: Option<&'a str>,
    sort: Option<&'a str>,
    team_id: Option<i32>,
    user_id: Option<i32>,
}

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
pub fn search(req: &mut dyn RequestExt) -> EndpointResult {
    use diesel::sql_types::{Bool, Text};

    // Don't require that authentication succeed, because it's only necessary
    // if the "following" param is set.
    let authenticated_user: AppResult<AuthenticatedUser> = req.authenticate();
    let conn = req.db_read_only()?;

    let params = serde_urlencoded::from_str::<QueryParams<'_>>(req.query_string().unwrap_or(""))
        .map_err(|e| bad_request(&e))?;

    let sort = params.sort;
    let include_yanked = params.include_yanked.map(|s| s == "yes").unwrap_or(true);

    let selection = (
        ALL_COLUMNS,
        false.into_sql::<Bool>(),
        recent_crate_downloads::downloads.nullable(),
    );
    let mut query = crates::table
        .left_join(recent_crate_downloads::table)
        .select(selection)
        .into_boxed();

    if let Some(q_string) = params.q {
        if !q_string.is_empty() {
            let sort = params.sort.unwrap_or("relevance");

            let q = sql::<TsQuery>("plainto_tsquery('english', ")
                .bind::<Text, _>(q_string)
                .sql(")");
            query = query.filter(
                q.clone()
                    .matches(crates::textsearchable_index_col)
                    .or(Crate::loosly_matches_name(&q_string)),
            );

            query = query.select((
                ALL_COLUMNS,
                Crate::with_name(q_string),
                recent_crate_downloads::downloads.nullable(),
            ));
            query = query.order(Crate::with_name(q_string).desc());

            if sort == "relevance" {
                let rank = ts_rank_cd(crates::textsearchable_index_col, q);
                query = query.then_order_by(rank.desc())
            }
        }
    }

    if let Some(cat) = params.category {
        query = query.filter(
            crates::id.eq_any(
                crates_categories::table
                    .select(crates_categories::crate_id)
                    .inner_join(categories::table)
                    .filter(
                        categories::slug
                            .eq(cat)
                            .or(categories::slug.like(format!("{}::%", cat))),
                    ),
            ),
        );
    }

    if let Some(kws) = params.all_keywords {
        use diesel::sql_types::Array;
        sql_function!(#[aggregate] fn array_agg<T>(x: T) -> Array<T>);

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
    } else if let Some(kw) = params.keyword {
        query = query.filter(
            crates::id.eq_any(
                crates_keywords::table
                    .select(crates_keywords::crate_id)
                    .inner_join(keywords::table)
                    .filter(crate::lower(keywords::keyword).eq(crate::lower(kw))),
            ),
        );
    } else if let Some(letter) = params.letter {
        let pattern = format!(
            "{}%",
            letter
                .chars()
                .next()
                .chain_error(|| bad_request("letter value must contain 1 character"))?
                .to_lowercase()
                .collect::<String>()
        );
        query = query.filter(canon_crate_name(crates::name).like(pattern));
    } else if let Some(user_id) = params.user_id {
        query = query.filter(
            crates::id.eq_any(
                CrateOwner::by_owner_kind(OwnerKind::User)
                    .select(crate_owners::crate_id)
                    .filter(crate_owners::owner_id.eq(user_id)),
            ),
        );
    } else if let Some(team_id) = params.team_id {
        query = query.filter(
            crates::id.eq_any(
                CrateOwner::by_owner_kind(OwnerKind::Team)
                    .select(crate_owners::crate_id)
                    .filter(crate_owners::owner_id.eq(team_id)),
            ),
        );
    } else if params.following.is_some() {
        let user_id = authenticated_user?.user_id();
        query = query.filter(
            crates::id.eq_any(
                follows::table
                    .select(follows::crate_id)
                    .filter(follows::user_id.eq(user_id)),
            ),
        );
    }

    if !include_yanked {
        query = query.filter(exists(
            versions::table
                .filter(versions::crate_id.eq(crates::id))
                .filter(versions::yanked.eq(false)),
        ));
    }

    if sort == Some("downloads") {
        query = query.then_order_by(crates::downloads.desc())
    } else if sort == Some("recent-downloads") {
        query = query.then_order_by(recent_crate_downloads::downloads.desc().nulls_last())
    } else if sort == Some("recent-updates") {
        query = query.order(crates::updated_at.desc());
    } else if sort == Some("new") {
        query = query.order(crates::created_at.desc());
    } else {
        query = query.then_order_by(crates::name.asc())
    }

    let data: Paginated<(Crate, bool, Option<i64>)> = query
        .paginate(&req.query_string().unwrap_or(""))?
        .load(&*conn)?;
    let total = data.total();

    let next_page = data
        .next_page()
        .map(|p| req.query_with_params(to_page_map(p)));
    let prev_page = data
        .prev_page()
        .map(|p| req.query_with_params(to_page_map(p)));

    let perfect_matches = data.iter().map(|&(_, b, _)| b).collect::<Vec<_>>();
    let recent_downloads = data
        .iter()
        .map(|&(_, _, s)| s.unwrap_or(0))
        .collect::<Vec<_>>();
    let crates = data.into_iter().map(|(c, _, _)| c).collect::<Vec<_>>();

    let versions: Vec<Version> = crates.versions().load(&*conn)?;
    let versions = versions
        .grouped_by(&crates)
        .into_iter()
        .map(|versions| Version::top(versions.into_iter().map(|v| (v.created_at, v.num))));

    let badges: Vec<CrateBadge> = CrateBadge::belonging_to(&crates)
        .select((badges::crate_id, badges::all_columns))
        .load(&*conn)?;
    let badges = badges
        .grouped_by(&crates)
        .into_iter()
        .map(|badges| badges.into_iter().map(|cb| cb.badge).collect());

    let crates = versions
        .zip(crates)
        .zip(perfect_matches)
        .zip(recent_downloads)
        .zip(badges)
        .map(
            |((((max_version, krate), perfect_match), recent_downloads), badges)| {
                krate.minimal_encodable(
                    &max_version,
                    Some(badges),
                    perfect_match,
                    Some(recent_downloads),
                )
            },
        )
        .collect();

    #[derive(Serialize)]
    struct R {
        crates: Vec<EncodableCrate>,
        meta: Meta,
    }
    #[derive(Serialize)]
    struct Meta {
        total: Option<i64>,
        next_page: Option<String>,
        prev_page: Option<String>,
    }

    Ok(req.json(&R {
        crates,
        meta: Meta {
            total,
            next_page,
            prev_page,
        },
    }))
}

fn to_page_map(page: u32) -> IndexMap<String, String> {
    let mut opts = IndexMap::new();
    opts.insert("page".into(), page.to_string());
    opts
}

diesel_infix_operator!(Contains, "@>");
