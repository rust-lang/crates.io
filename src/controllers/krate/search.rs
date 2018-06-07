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
    if sort == "downloads" {
        query = query.then_order_by(crates::downloads.desc())
    } else if sort == "recent-downloads" {
        query = query.then_order_by(recent_crate_downloads::downloads.desc().nulls_last())
    } else {
        query = query.then_order_by(crates::name.asc())
    }
    // The database query returns a tuple within a tuple, with the root
    // tuple containing 3 items.
    let data = query
        .paginate(limit, offset)
        .load::<((Crate, bool, Option<i64>), i64)>(&*conn)
        .unwrap();
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
    use models::{Category, CrateDownload, CrateOwner, Follow, Keyword, NewCrate, NewTeam, NewUser,
                 Team, User};
    use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

    static NEXT_ID: AtomicUsize = ATOMIC_USIZE_INIT;

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
        categories: Vec<&'a str>,
        keywords: Vec<&'a str>,
        team: Option<&'a Team>,
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
                categories: Vec::new(),
                keywords: Vec::new(),
                team: None,
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

        fn category(mut self, category: &'a str) -> Self {
            self.categories.push(category);
            self
        }

        fn keyword(mut self, keyword: &'a str) -> Self {
            self.keywords.push(keyword);
            self
        }

        fn team(mut self, team: &'a Team) -> Self {
            self.team = Some(&team);
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

            if let Some(team) = self.team {
                let crate_owner = CrateOwner {
                    crate_id: krate.id,
                    owner_id: team.id,
                    created_by: self.owner_id,
                    owner_kind: 1, // Team owner kind is 1 according to owner.rs
                };
                insert_into(crate_owners::table)
                    .values(&crate_owner)
                    .on_conflict(crate_owners::table.primary_key())
                    .do_update()
                    .set(crate_owners::deleted.eq(false))
                    .execute(connection)?;
            }

            if self.categories.len() > 0 {
                Category::update_crate(&connection, &krate, &self.categories)?;
            }

            if !self.keywords.is_empty() {
                Keyword::update_crate(connection, &krate, &self.keywords)?;
            }

            Ok(krate)
        }
    }

    fn create_user(conn: &PgConnection) -> User {
        let user_id = NEXT_ID.fetch_add(1, Ordering::SeqCst) as i32;
        NewUser::new(user_id, "login", None, None, None, "access_token")
            .create_or_update(conn)
            .unwrap()
    }

    fn create_team(conn: &PgConnection, login: &str) -> Team {
        let team = NewTeam {
            github_id: NEXT_ID.fetch_add(1, Ordering::SeqCst) as i32,
            login: login,
            name: None,
            avatar: None,
        };
        return team.create_or_update(conn).unwrap();
    }

    #[test]
    fn no_parameters_or_sorting_returns_in_alphabetic_order() {
        let db_connection = conn();
        let user = create_user(&db_connection);
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
        let user = create_user(&db_connection);
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
    fn no_parameters_and_sorting_by_recent_downloads_returns_crates_by_descending_order_of_recent_downloads(
) {
        let db_connection = conn();
        let user = create_user(&db_connection);
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

    #[test]
    fn query_parameter_is_empty_returns_all_crates() {
        let db_connection = conn();
        let user = create_user(&db_connection);
        CrateBuilder::new("100 recent downloads", user.id)
            .build(&db_connection)
            .unwrap();
        CrateBuilder::new("50 recent downloads", user.id)
            .build(&db_connection)
            .unwrap();
        CrateBuilder::new("300 recent downloads", user.id)
            .build(&db_connection)
            .unwrap();

        let sort = "";
        let mut params: HashMap<String, String> = HashMap::new();
        params.insert("q".to_string(), String::new());
        let list_of_krates =
            execute_search(&db_connection, 0, 100, params, sort.to_string(), user.id);

        assert_eq!(list_of_krates.len(), 3);
    }

    #[test]
    fn query_parameter_is_not_empty_returns_crates_that_match_query() {
        let db_connection = conn();
        let user = create_user(&db_connection);
        CrateBuilder::new("Found Crate", user.id)
            .build(&db_connection)
            .unwrap();
        CrateBuilder::new("Not found", user.id)
            .build(&db_connection)
            .unwrap();
        CrateBuilder::new("Another not found", user.id)
            .build(&db_connection)
            .unwrap();

        let sort = "";
        let mut params: HashMap<String, String> = HashMap::new();
        params.insert("q".to_string(), "Crate".to_string());
        let list_of_krates =
            execute_search(&db_connection, 0, 100, params, sort.to_string(), user.id);

        assert_eq!(list_of_krates.len(), 1);
        assert_eq!(
            list_of_krates.get(0).unwrap().name,
            "Found Crate".to_string()
        );
    }

    #[test]
    #[ignore]
    fn when_searching_by_category_returns_crates_that_match_category() {
        let db_connection = conn();
        let user = create_user(&db_connection);
        CrateBuilder::new("Found Crate", user.id)
            .category(&"category1")
            .build(&db_connection)
            .unwrap();

        CrateBuilder::new("Not found", user.id)
            .category(&"category1")
            .category(&"category2")
            .build(&db_connection)
            .unwrap();

        CrateBuilder::new("Another not found", user.id)
            .category(&"category2")
            .build(&db_connection)
            .unwrap();

        let sort = "";
        let mut params: HashMap<String, String> = HashMap::new();
        params.insert("category".to_string(), "category1".to_string());
        let list_of_krates =
            execute_search(&db_connection, 0, 100, params, sort.to_string(), user.id);

        assert_eq!(list_of_krates.len(), 2);
        assert_eq!(
            list_of_krates.get(0).unwrap().name,
            "Found Crate".to_string()
        );
    }

    #[test]
    fn when_searching_by_keyword_returns_crates_that_match_keyword() {
        let db_connection = conn();
        let user = create_user(&db_connection);
        CrateBuilder::new("Found Crate", user.id)
            .keyword(&"found")
            .build(&db_connection)
            .unwrap();

        CrateBuilder::new("Not found", user.id)
            .keyword(&"not found")
            .build(&db_connection)
            .unwrap();

        CrateBuilder::new("Another not found", user.id)
            .keyword(&"not found")
            .build(&db_connection)
            .unwrap();

        let sort = "";
        let mut params: HashMap<String, String> = HashMap::new();
        params.insert("keyword".to_string(), "found".to_string());
        let list_of_krates =
            execute_search(&db_connection, 0, 100, params, sort.to_string(), user.id);

        assert_eq!(list_of_krates.len(), 1);
        assert_eq!(
            list_of_krates.get(0).unwrap().name,
            "Found Crate".to_string()
        );
    }

    #[test]
    #[ignore]
    fn when_searching_by_letter_in_name_returns_crates_that_match_the_letters() {
        let db_connection = conn();
        let user = create_user(&db_connection);
        CrateBuilder::new("Found Crate", user.id)
            .build(&db_connection)
            .unwrap();

        CrateBuilder::new("Not found", user.id)
            .build(&db_connection)
            .unwrap();

        CrateBuilder::new("Another not found", user.id)
            .build(&db_connection)
            .unwrap();

        let sort = "";
        let mut params: HashMap<String, String> = HashMap::new();
        params.insert("letter".to_string(), "c".to_string());
        let list_of_krates =
            execute_search(&db_connection, 0, 100, params, sort.to_string(), user.id);

        assert_eq!(list_of_krates.len(), 1);
        assert_eq!(
            list_of_krates.get(0).unwrap().name,
            "Found Crate".to_string()
        );
    }

    #[test]
    fn when_searching_by_user_returns_crates_that_where_create_by_the_user() {
        let db_connection = conn();
        let user = create_user(&db_connection);
        let user1 = create_user(&db_connection);
        let user2 = create_user(&db_connection);
        CrateBuilder::new("Found Crate", user.id)
            .build(&db_connection)
            .unwrap();

        CrateBuilder::new("Not found", user1.id)
            .build(&db_connection)
            .unwrap();

        CrateBuilder::new("Another not found", user2.id)
            .build(&db_connection)
            .unwrap();

        let sort = "";
        let mut params: HashMap<String, String> = HashMap::new();
        params.insert("user_id".to_string(), user.id.to_string());
        let list_of_krates =
            execute_search(&db_connection, 0, 100, params, sort.to_string(), user.id);

        assert_eq!(list_of_krates.len(), 1);
        assert_eq!(
            list_of_krates.get(0).unwrap().name,
            "Found Crate".to_string()
        );
    }

    #[test]
    fn when_searching_by_team_returns_crates_that_where_create_by_the_team() {
        let db_connection = conn();
        let user = create_user(&db_connection);
        let team = create_team(&db_connection, "team@ecrates.com");
        CrateBuilder::new("Found Crate", user.id)
            .team(&team)
            .build(&db_connection)
            .unwrap();

        CrateBuilder::new("Not found", user.id)
            .build(&db_connection)
            .unwrap();

        CrateBuilder::new("Another not found", user.id)
            .build(&db_connection)
            .unwrap();

        let sort = "";
        let mut params: HashMap<String, String> = HashMap::new();
        params.insert("team_id".to_string(), team.id.to_string());
        let list_of_krates =
            execute_search(&db_connection, 0, 100, params, sort.to_string(), team.id);

        assert_eq!(list_of_krates.len(), 1);
        assert_eq!(
            list_of_krates.get(0).unwrap().name,
            "Found Crate".to_string()
        );
    }

    #[test]
    fn when_searching_by_crates_user_follows_returns_all_crates_the_user_follows() {
        use diesel::insert_into;
        let db_connection = conn();
        let user = create_user(&db_connection);
        let user1 = create_user(&db_connection);
        let krate = CrateBuilder::new("Found Crate", user.id)
            .build(&db_connection)
            .unwrap();

        CrateBuilder::new("Not found", user.id)
            .build(&db_connection)
            .unwrap();

        CrateBuilder::new("Another not found", user.id)
            .build(&db_connection)
            .unwrap();
        let follow = Follow {
            user_id: user1.id,
            crate_id: krate.id,
        };
        insert_into(follows::table)
            .values(&follow)
            .on_conflict_do_nothing()
            .execute(&db_connection)
            .unwrap();

        let sort = "";
        let mut params: HashMap<String, String> = HashMap::new();
        params.insert("following".to_string(), "".to_string());
        let list_of_krates =
            execute_search(&db_connection, 0, 100, params, sort.to_string(), user1.id);

        assert_eq!(list_of_krates.len(), 1);
        assert_eq!(
            list_of_krates.get(0).unwrap().name,
            "Found Crate".to_string()
        );
    }
}
