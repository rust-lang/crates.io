//! Endpoint for searching and discovery functionality

use diesel::dsl::*;
use diesel::sql_types::Array;
use diesel_full_text_search::*;
use indexmap::IndexMap;

use crate::controllers::cargo_prelude::*;
use crate::controllers::helpers::Paginate;
use crate::models::{
    Crate, CrateBadge, CrateOwner, CrateVersions, OwnerKind, TopVersions, Version,
};
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
pub fn search(req: &mut dyn RequestExt) -> EndpointResult {
    use diesel::sql_types::{Bool, Text};

    let params = req.query();
    let sort = params.get("sort").map(|s| &**s);
    let include_yanked = params
        .get("include_yanked")
        .map(|s| s == "yes")
        .unwrap_or(true);

    let selection = (
        ALL_COLUMNS,
        false.into_sql::<Bool>(),
        recent_crate_downloads::downloads.nullable(),
    );
    let mut query = crates::table
        .left_join(recent_crate_downloads::table)
        .select(selection)
        .into_boxed();

    let mut supports_seek = true;

    if let Some(q_string) = params.get("q") {
        // Searching with a query string always puts the exact match at the start of the results,
        // so we can't support seek-based pagination with it.
        supports_seek = false;

        if !q_string.is_empty() {
            let sort = params.get("sort").map(|s| &**s).unwrap_or("relevance");

            let q = sql::<TsQuery>("plainto_tsquery('english', ")
                .bind::<Text, _>(q_string)
                .sql(")");
            query = query.filter(
                q.clone()
                    .matches(crates::textsearchable_index_col)
                    .or(Crate::loosly_matches_name(q_string)),
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

    if let Some(cat) = params.get("category") {
        // Calculating the total number of results with filters is not supported yet.
        supports_seek = false;

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

    if let Some(kws) = params.get("all_keywords") {
        // Calculating the total number of results with filters is not supported yet.
        supports_seek = false;

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
    } else if let Some(kw) = params.get("keyword") {
        // Calculating the total number of results with filters is not supported yet.
        supports_seek = false;

        query = query.filter(
            crates::id.eq_any(
                crates_keywords::table
                    .select(crates_keywords::crate_id)
                    .inner_join(keywords::table)
                    .filter(lower(keywords::keyword).eq(lower(kw))),
            ),
        );
    } else if let Some(letter) = params.get("letter") {
        // Calculating the total number of results with filters is not supported yet.
        supports_seek = false;

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
    } else if let Some(user_id) = params.get("user_id").and_then(|s| s.parse::<i32>().ok()) {
        // Calculating the total number of results with filters is not supported yet.
        supports_seek = false;

        query = query.filter(
            crates::id.eq_any(
                CrateOwner::by_owner_kind(OwnerKind::User)
                    .select(crate_owners::crate_id)
                    .filter(crate_owners::owner_id.eq(user_id)),
            ),
        );
    } else if let Some(team_id) = params.get("team_id").and_then(|s| s.parse::<i32>().ok()) {
        // Calculating the total number of results with filters is not supported yet.
        supports_seek = false;

        query = query.filter(
            crates::id.eq_any(
                CrateOwner::by_owner_kind(OwnerKind::Team)
                    .select(crate_owners::crate_id)
                    .filter(crate_owners::owner_id.eq(team_id)),
            ),
        );
    } else if params.get("following").is_some() {
        // Calculating the total number of results with filters is not supported yet.
        supports_seek = false;

        let user_id = req.authenticate()?.user_id();
        query = query.filter(
            crates::id.eq_any(
                follows::table
                    .select(follows::crate_id)
                    .filter(follows::user_id.eq(user_id)),
            ),
        );
    } else if params.get("ids[]").is_some() {
        // Calculating the total number of results with filters is not supported yet.
        supports_seek = false;

        let query_bytes = req.query_string().unwrap_or("").as_bytes();
        let ids: Vec<_> = url::form_urlencoded::parse(query_bytes)
            .filter(|(key, _)| key == "ids[]")
            .map(|(_, value)| value.to_string())
            .collect();

        query = query.filter(crates::name.eq(any(ids)));
    }

    if !include_yanked {
        // Calculating the total number of results with filters is not supported yet.
        supports_seek = false;

        query = query.filter(exists(
            versions::table
                .filter(versions::crate_id.eq(crates::id))
                .filter(versions::yanked.eq(false)),
        ));
    }

    if sort == Some("downloads") {
        // Custom sorting is not supported yet with seek.
        supports_seek = false;

        query = query.then_order_by(crates::downloads.desc())
    } else if sort == Some("recent-downloads") {
        // Custom sorting is not supported yet with seek.
        supports_seek = false;

        query = query.then_order_by(recent_crate_downloads::downloads.desc().nulls_last())
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
        .limit_page_numbers(req.app().clone())
        .enable_seek(supports_seek)
        .gather(req)?;
    let conn = req.db_read_only()?;

    let (explicit_page, seek) = match pagination.page.clone() {
        Page::Numeric(_) => (true, None),
        Page::Seek(s) => (false, Some(s.decode::<i32>()?)),
        Page::Unspecified => (false, None),
    };

    // To avoid breaking existing users, seek-based pagination is only used if an explicit page has
    // not been provided. This way clients relying on meta.next_page will use the faster seek-based
    // paginations, while client hardcoding pages handling will use the slower offset-based code.
    let (total, next_page, prev_page, data, conn) = if supports_seek && !explicit_page {
        // Equivalent of:
        // `WHERE name > (SELECT name FROM crates WHERE id = $1) LIMIT $2`
        query = query.limit(pagination.per_page as i64);
        if let Some(seek) = seek {
            let crate_name: String = crates::table
                .find(seek)
                .select(crates::name)
                .get_result(&*conn)?;
            query = query.filter(crates::name.gt(crate_name));
        }

        // This does a full index-only scan over the crates table to gather how many crates were
        // published. Unfortunately on PostgreSQL counting the rows in a table requires scanning
        // the table, and the `total` field is part of the stable registries API.
        //
        // If this becomes a problem in the future the crates count could be denormalized, at least
        // for the filterless happy path.
        let total: i64 = crates::table.count().get_result(&*conn)?;

        let results: Vec<(Crate, bool, Option<i64>)> = query.load(&*conn)?;

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
        let query = query.pages_pagination(pagination);
        let data: Paginated<(Crate, bool, Option<i64>)> = query.load(&*conn)?;
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

    let versions: Vec<Version> = crates.versions().load(&*conn)?;
    let versions = versions
        .grouped_by(&crates)
        .into_iter()
        .map(TopVersions::from_versions);

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
                EncodableCrate::from_minimal(
                    krate,
                    &max_version,
                    Some(badges),
                    perfect_match,
                    Some(recent_downloads),
                )
            },
        )
        .collect::<Vec<_>>();

    Ok(req.json(&json!({
        "crates": crates,
        "meta": {
            "total": total,
            "next_page": next_page,
            "prev_page": prev_page,
        },
    })))
}

diesel_infix_operator!(Contains, "@>");
