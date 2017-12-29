//! Endpoint for searching and discovery functionality

use conduit::{Request, Response};
use diesel::prelude::*;
use diesel_full_text_search::*;

use db::RequestTransaction;
use owner::OwnerKind;
use pagination::Paginate;
use schema::*;
use user::RequestUser;
use util::{CargoResult, RequestUtils};
use {Badge, Version};

use super::{canon_crate_name, Crate, EncodableCrate, ALL_COLUMNS};

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
pub fn search(req: &mut Request) -> CargoResult<Response> {
    use diesel::dsl::*;
    use diesel::types::{BigInt, Bool, Nullable};

    let conn = req.db_conn()?;
    let (offset, limit) = req.pagination(10, 100)?;
    let params = req.query();
    let sort = params
        .get("sort")
        .map(|s| &**s)
        .unwrap_or("recent-downloads");

    let recent_downloads = sql::<Nullable<BigInt>>("SUM(crate_downloads.downloads)");

    let mut query = crates::table
        .left_join(
            crate_downloads::table.on(crates::id
                .eq(crate_downloads::crate_id)
                .and(crate_downloads::date.gt(date(now - 90.days())))),
        )
        .group_by(crates::id)
        .select((
            ALL_COLUMNS,
            false.into_sql::<Bool>(),
            recent_downloads.clone(),
        ))
        .into_boxed();

    if sort == "downloads" {
        query = query.order(crates::downloads.desc())
    } else if sort == "recent-downloads" {
        query = query.order(recent_downloads.clone().desc().nulls_last())
    } else {
        query = query.order(crates::name.asc())
    }

    if let Some(q_string) = params.get("q") {
        let sort = params.get("sort").map(|s| &**s).unwrap_or("relevance");
        let q = plainto_tsquery(q_string);
        query = query.filter(
            q.matches(crates::textsearchable_index_col)
                .or(Crate::with_name(q_string)),
        );

        query = query.select((
            ALL_COLUMNS,
            Crate::with_name(q_string),
            recent_downloads.clone(),
        ));
        let perfect_match = Crate::with_name(q_string).desc();
        if sort == "downloads" {
            query = query.order((perfect_match, crates::downloads.desc()));
        } else if sort == "recent-downloads" {
            query = query.order((perfect_match, recent_downloads.clone().desc().nulls_last()));
        } else {
            let rank = ts_rank_cd(crates::textsearchable_index_col, q);
            query = query.order((perfect_match, rank.desc()))
        }
    }

    if let Some(cat) = params.get("category") {
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

    if let Some(kw) = params.get("keyword") {
        query = query.filter(
            crates::id.eq_any(
                crates_keywords::table
                    .select(crates_keywords::crate_id)
                    .inner_join(keywords::table)
                    .filter(::lower(keywords::keyword).eq(::lower(kw))),
            ),
        );
    } else if let Some(letter) = params.get("letter") {
        let pattern = format!(
            "{}%",
            letter
                .chars()
                .next()
                .unwrap()
                .to_lowercase()
                .collect::<String>()
        );
        query = query.filter(canon_crate_name(crates::name).like(pattern));
    } else if let Some(user_id) = params.get("user_id").and_then(|s| s.parse::<i32>().ok()) {
        query = query.filter(
            crates::id.eq_any(
                crate_owners::table
                    .select(crate_owners::crate_id)
                    .filter(crate_owners::owner_id.eq(user_id))
                    .filter(crate_owners::deleted.eq(false))
                    .filter(crate_owners::owner_kind.eq(OwnerKind::User as i32)),
            ),
        );
    } else if let Some(team_id) = params.get("team_id").and_then(|s| s.parse::<i32>().ok()) {
        query = query.filter(
            crates::id.eq_any(
                crate_owners::table
                    .select(crate_owners::crate_id)
                    .filter(crate_owners::owner_id.eq(team_id))
                    .filter(crate_owners::deleted.eq(false))
                    .filter(crate_owners::owner_kind.eq(OwnerKind::Team as i32)),
            ),
        );
    } else if params.get("following").is_some() {
        query = query.filter(
            crates::id.eq_any(
                follows::table
                    .select(follows::crate_id)
                    .filter(follows::user_id.eq(req.user()?.id)),
            ),
        );
    }

    // The database query returns a tuple within a tuple , with the root
    // tuple containing 3 items.
    let data = query
        .paginate(limit, offset)
        .load::<((Crate, bool, Option<i64>), i64)>(&*conn)?;
    let total = data.first().map(|&(_, t)| t).unwrap_or(0);
    let crates = data.iter()
        .map(|&((ref c, _, _), _)| c.clone())
        .collect::<Vec<_>>();
    let perfect_matches = data.clone()
        .into_iter()
        .map(|((_, b, _), _)| b)
        .collect::<Vec<_>>();
    let recent_downloads = data.clone()
        .into_iter()
        .map(|((_, _, s), _)| s.unwrap_or(0))
        .collect::<Vec<_>>();

    let versions = Version::belonging_to(&crates)
        .load::<Version>(&*conn)?
        .grouped_by(&crates)
        .into_iter()
        .map(|versions| Version::max(versions.into_iter().map(|v| v.num)));

    let crates = versions
        .zip(crates)
        .zip(perfect_matches)
        .zip(recent_downloads)
        .map(
            |(((max_version, krate), perfect_match), recent_downloads)| {
                // FIXME: If we add crate_id to the Badge enum we can eliminate
                // this N+1
                let badges = badges::table
                    .filter(badges::crate_id.eq(krate.id))
                    .load::<Badge>(&*conn)?;
                Ok(krate.minimal_encodable(
                    &max_version,
                    Some(badges),
                    perfect_match,
                    Some(recent_downloads),
                ))
            },
        )
        .collect::<Result<_, ::diesel::result::Error>>()?;

    #[derive(Serialize)]
    struct R {
        crates: Vec<EncodableCrate>,
        meta: Meta,
    }
    #[derive(Serialize)]
    struct Meta {
        total: i64,
    }

    Ok(req.json(&R {
        crates: crates,
        meta: Meta { total: total },
    }))
}
