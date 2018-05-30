//! Endpoint for searching and discovery functionality

use diesel_full_text_search::*;

use controllers::helpers::Paginate;
use controllers::prelude::*;
use models::{Crate, CrateBadge, OwnerKind, Version};
use schema::*;
use views::EncodableCrate;

use models::krate::{canon_crate_name, ALL_COLUMNS};

use std::collections::HashMap;

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
    let conn = req.db_conn()?;
    let (offset, limit) = req.pagination(10, 100)?;
    let params = req.query();
    let temp_params = params.clone();

    let sort = temp_params
        .get("sort")
        .map(|s| &**s)
        .unwrap_or("recent-downloads")
        .clone();
    let current_user_id = req.user()?.id;

    let crates = execute_search(
        &conn,
        offset,
        limit,
        params,
        sort.to_string(),
        current_user_id,
    );
    let total = crates.len() as i64;

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
        crates,
        meta: Meta { total },
    }))
}

fn execute_search(
    conn: &PgConnection,
    offset: i64,
    limit: i64,
    params: HashMap<String, String>,
    sort: String,
    current_user_id: i32,
) -> Vec<EncodableCrate> {
    use diesel::sql_types::Bool;

    let mut query = crates::table
        .left_join(recent_crate_downloads::table)
        .select((
            ALL_COLUMNS,
            false.into_sql::<Bool>(),
            recent_crate_downloads::downloads.nullable(),
        ))
        .into_boxed();
    if let Some(q_string) = params.get("q") {
        if !q_string.is_empty() {
            let sort = params.get("sort").map(|s| &**s).unwrap_or("relevance");
            let q = plainto_tsquery(q_string);
            query = query.filter(
                q.matches(crates::textsearchable_index_col)
                    .or(Crate::with_name(q_string)),
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
                    .filter(follows::user_id.eq(current_user_id)),
            ),
        );
    }
    println!("Sort value is: {}", sort);
    if sort == "downloads" {
        println!("Downloads");
        query = query.then_order_by(crates::downloads.desc())
    } else if sort == "recent-downloads" {
        println!("Recent-Downloads");
        query = query.then_order_by(recent_crate_downloads::downloads.desc().nulls_last())
    } else {
        println!("Alphabetic");
        query = query.then_order_by(crates::name.asc())
    }
    // The database query returns a tuple within a tuple, with the root
    // tuple containing 3 items.
    let data = query
        .paginate(limit, offset)
        .load::<((Crate, bool, Option<i64>), i64)>(&*conn)
        .unwrap();
    let _total = data.first().map(|&(_, t)| t).unwrap_or(0);
    let perfect_matches = data.iter().map(|&((_, b, _), _)| b).collect::<Vec<_>>();
    let recent_downloads = data.iter()
        .map(|&((_, _, s), _)| s.unwrap_or(0))
        .collect::<Vec<_>>();
    let crates = data.into_iter().map(|((c, _, _), _)| c).collect::<Vec<_>>();
    let versions = Version::belonging_to(&crates)
        .load::<Version>(&*conn)
        .unwrap()
        .grouped_by(&crates)
        .into_iter()
        .map(|versions| Version::max(versions.into_iter().map(|v| v.num)));
    let badges = CrateBadge::belonging_to(&crates)
        .select((badges::crate_id, badges::all_columns))
        .load::<CrateBadge>(&*conn)
        .unwrap()
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
    return crates;
}

#[cfg(test)]
mod test {
    extern crate conduit_test;
    extern crate semver;

    use std::collections::HashMap;

    use super::*;
    use std::env;
    extern crate chrono;
    use chrono::Utc;
    use models::{CrateDownload, NewCrate, NewUser, User};

    fn conn() -> PgConnection {
        let database_url =
            env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set to run tests");
        let conn = PgConnection::establish(&database_url).unwrap();
        conn.begin_test_transaction().unwrap();
        conn
    }

    pub struct CrateBuilder<'a> {
        owner_id: i32,
        krate: NewCrate<'a>,
        downloads: Option<i32>,
        recent_downloads: Option<i32>,
    }

    impl<'a> CrateBuilder<'a> {
        fn new(name: &str, owner_id: i32) -> CrateBuilder {
            CrateBuilder {
                owner_id: owner_id,
                krate: NewCrate {
                    name: name,
                    ..NewCrate::default()
                },
                downloads: None,
                recent_downloads: None,
            }
        }

        fn downloads(mut self, downloads: i32) -> Self {
            self.downloads = Some(downloads);
            self
        }

        fn recent_downloads(mut self, recent_downloads: i32) -> Self {
            self.recent_downloads = Some(recent_downloads);
            self
        }

        fn build(self, connection: &PgConnection) -> CargoResult<Crate> {
            use diesel::{insert_into, select, update};

            let mut krate = self.krate
                .create_or_update(connection, None, self.owner_id)?;

            // Since we are using `NewCrate`, we can't set all the
            // crate properties in a single DB call.

            let old_downloads = self.downloads.unwrap_or(0) - self.recent_downloads.unwrap_or(0);
            let now = Utc::now();
            let old_date = now.naive_utc().date() - chrono::Duration::days(91);

            if let Some(downloads) = self.downloads {
                let crate_download = CrateDownload {
                    crate_id: krate.id,
                    downloads: old_downloads,
                    date: old_date,
                };

                insert_into(crate_downloads::table)
                    .values(&crate_download)
                    .execute(connection)?;
                krate.downloads = downloads;
                update(&krate).set(&krate).execute(connection)?;
            }
            if self.recent_downloads.is_some() {
                let crate_download = CrateDownload {
                    crate_id: krate.id,
                    downloads: self.recent_downloads.unwrap(),
                    date: now.naive_utc().date(),
                };

                insert_into(crate_downloads::table)
                    .values(&crate_download)
                    .execute(connection)?;

                no_arg_sql_function!(refresh_recent_crate_downloads, ());
                select(refresh_recent_crate_downloads).execute(connection)?;
            }
            Ok(krate)
        }
    }

    fn user(conn: &PgConnection) -> User {
        NewUser::new(2, "login", None, None, None, "access_token")
            .create_or_update(conn)
            .unwrap()
    }

    #[test]
    fn no_parameters_or_sorting_returns_in_alphabetic_order() {
        let db_connection = conn();
        let user = user(&db_connection);
        let krate1 = CrateBuilder::new("1 first crate", user.id)
            .build(&db_connection)
            .unwrap();
        let krate3 = CrateBuilder::new("third crate", user.id)
            .build(&db_connection)
            .unwrap();
        let krate2 = CrateBuilder::new("second crate", user.id)
            .build(&db_connection)
            .unwrap();

        let sort = "";
        let params: HashMap<String, String> = HashMap::new();
        let list_of_krates =
            execute_search(&db_connection, 0, 100, params, sort.to_string(), user.id);

        assert_eq!(list_of_krates.len(), 3);
        assert_eq!(list_of_krates.get(0).unwrap().name, krate1.name);
        assert_eq!(list_of_krates.get(1).unwrap().name, krate2.name);
        assert_eq!(list_of_krates.get(2).unwrap().name, krate3.name);
    }

    #[test]
    fn no_parameters_and_sorting_by_downloads_returns_crates_by_descending_order_of_downloads() {
        let db_connection = conn();
        let user = user(&db_connection);
        let krate2 = CrateBuilder::new("100 Downloads", user.id)
            .downloads(100)
            .build(&db_connection)
            .unwrap();
        let krate3 = CrateBuilder::new("50 Downloads", user.id)
            .downloads(50)
            .build(&db_connection)
            .unwrap();
        let krate1 = CrateBuilder::new("300 Downloads", user.id)
            .downloads(300)
            .build(&db_connection)
            .unwrap();

        let sort = "downloads";
        let params: HashMap<String, String> = HashMap::new();
        let list_of_krates =
            execute_search(&db_connection, 0, 100, params, sort.to_string(), user.id);

        assert_eq!(list_of_krates.len(), 3);
        assert_eq!(list_of_krates.get(0).unwrap().name, krate1.name);
        assert_eq!(list_of_krates.get(1).unwrap().name, krate2.name);
        assert_eq!(list_of_krates.get(2).unwrap().name, krate3.name);
    }

    #[test]
    fn no_parameters_and_sorting_by_recent_downloads_returns_crates_by_descending_order_of_downloads(
) {
        let db_connection = conn();
        let user = user(&db_connection);
        let krate2 = CrateBuilder::new("100 recent downloads", user.id)
            .downloads(5000)
            .recent_downloads(100)
            .build(&db_connection)
            .unwrap();
        let krate3 = CrateBuilder::new("50 recent downloads", user.id)
            .downloads(50)
            .recent_downloads(50)
            .build(&db_connection)
            .unwrap();
        let krate1 = CrateBuilder::new("300 recent downloads", user.id)
            .downloads(1000)
            .recent_downloads(300)
            .build(&db_connection)
            .unwrap();

        let sort = "recent-downloads";
        let params: HashMap<String, String> = HashMap::new();
        let list_of_krates =
            execute_search(&db_connection, 0, 100, params, sort.to_string(), user.id);

        assert_eq!(list_of_krates.len(), 3);
        assert_eq!(list_of_krates.get(0).unwrap().name, krate1.name);
        assert_eq!(list_of_krates.get(1).unwrap().name, krate2.name);
        assert_eq!(list_of_krates.get(2).unwrap().name, krate3.name);
    }
}
