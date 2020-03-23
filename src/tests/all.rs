#![warn(clippy::all, rust_2018_idioms)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate serde_json;

use crate::util::{Bad, RequestHelper, TestApp};
use cargo_registry::{
    models::{Crate, CrateOwner, Dependency, NewCategory, NewTeam, NewUser, Team, User, Version},
    schema::crate_owners,
    views::{
        EncodableCategory, EncodableCategoryWithSubcategories, EncodableCrate, EncodableKeyword,
        EncodableOwner, EncodableVersion, GoodCrate,
    },
    App, Config, Env, Replica, Uploader,
};
use std::{
    borrow::Cow,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use conduit_test::MockRequest;
use diesel::prelude::*;
use reqwest::{blocking::Client, Proxy};

macro_rules! t {
    ($e:expr) => {
        match $e {
            Ok(e) => e,
            Err(m) => panic!("{} failed with: {}", stringify!($e), m),
        }
    };
}

mod authentication;
mod badge;
mod builders;
mod categories;
mod category;
mod dump_db;
mod git;
mod keyword;
mod krate;
mod owners;
mod read_only_mode;
mod record;
mod schema_details;
mod server;
mod team;
mod token;
mod user;
mod util;
mod version;

#[derive(Deserialize)]
pub struct CrateList {
    crates: Vec<EncodableCrate>,
    meta: CrateMeta,
}
#[derive(Deserialize)]
struct CrateMeta {
    total: i32,
    next_page: Option<String>,
    prev_page: Option<String>,
}
#[derive(Deserialize)]
pub struct CrateResponse {
    #[serde(rename = "crate")]
    krate: EncodableCrate,
    versions: Vec<EncodableVersion>,
    keywords: Vec<EncodableKeyword>,
}
#[derive(Deserialize)]
pub struct VersionResponse {
    version: EncodableVersion,
}
#[derive(Deserialize)]
pub struct OwnerTeamsResponse {
    teams: Vec<EncodableOwner>,
}
#[derive(Deserialize)]
pub struct OwnersResponse {
    users: Vec<EncodableOwner>,
}
#[derive(Deserialize)]
pub struct CategoryResponse {
    category: EncodableCategoryWithSubcategories,
}
#[derive(Deserialize)]
pub struct CategoryListResponse {
    categories: Vec<EncodableCategory>,
    meta: CategoryMeta,
}
#[derive(Deserialize)]
pub struct CategoryMeta {
    total: i32,
}
#[derive(Deserialize)]
pub struct OkBool {
    ok: bool,
}

fn app() -> (Arc<App>, conduit_middleware::MiddlewareBuilder) {
    build_app(simple_config(), None)
}

fn simple_config() -> Config {
    let uploader = Uploader::S3 {
        bucket: s3::Bucket::new(
            String::from("alexcrichton-test"),
            None,
            dotenv::var("S3_ACCESS_KEY").unwrap_or_default(),
            dotenv::var("S3_SECRET_KEY").unwrap_or_default(),
            // When testing we route all API traffic over HTTP so we can
            // sniff/record it, but everywhere else we use https
            "http",
        ),
        cdn: None,
    };

    Config {
        uploader,
        session_key: "test this has to be over 32 bytes long".to_string(),
        git_repo_checkout: git::checkout(),
        gh_client_id: dotenv::var("GH_CLIENT_ID").unwrap_or_default(),
        gh_client_secret: dotenv::var("GH_CLIENT_SECRET").unwrap_or_default(),
        db_url: env("TEST_DATABASE_URL"),
        replica_db_url: None,
        env: Env::Test,
        max_upload_size: 3000,
        max_unpack_size: 2000,
        mirror: Replica::Primary,
        // When testing we route all API traffic over HTTP so we can
        // sniff/record it, but everywhere else we use https
        api_protocol: String::from("http"),
        publish_rate_limit: Default::default(),
        blocked_traffic: Default::default(),
    }
}

fn build_app(
    config: Config,
    proxy: Option<String>,
) -> (Arc<App>, conduit_middleware::MiddlewareBuilder) {
    let client = if let Some(proxy) = proxy {
        let mut builder = Client::builder();
        builder = builder
            .proxy(Proxy::all(&proxy).expect("Unable to configure proxy with the provided URL"));
        Some(builder.build().expect("TLS backend cannot be initialized"))
    } else {
        None
    };

    let app = App::new(&config, client);
    t!(t!(app.primary_database.get()).begin_test_transaction());
    let app = Arc::new(app);
    let handler = cargo_registry::build_handler(Arc::clone(&app));
    (app, handler)
}

// Return the environment variable only if it has been defined
fn env(var: &str) -> String {
    match dotenv::var(var) {
        Ok(ref s) if s == "" => panic!("environment variable `{}` must not be empty", var),
        Ok(s) => s,
        _ => panic!(
            "environment variable `{}` must be defined and valid unicode",
            var
        ),
    }
}

fn req(method: conduit::Method, path: &str) -> MockRequest {
    let mut request = MockRequest::new(method, path);
    request.header("User-Agent", "conduit-test");
    request
}

fn ok_resp(r: &conduit::Response) -> bool {
    r.status.0 == 200
}

fn bad_resp(r: &mut conduit::Response) -> Option<Bad> {
    let bad = json::<Bad>(r);
    if bad.errors.is_empty() {
        return None;
    }
    Some(bad)
}

fn json<T>(r: &mut conduit::Response) -> T
where
    for<'de> T: serde::Deserialize<'de>,
{
    let mut data = Vec::new();
    r.body.write_body(&mut data).unwrap();
    let s = std::str::from_utf8(&data).unwrap();
    match serde_json::from_str(s) {
        Ok(t) => t,
        Err(e) => panic!("failed to decode: {:?}\n{}", e, s),
    }
}

static NEXT_GH_ID: AtomicUsize = AtomicUsize::new(0);

fn new_user(login: &str) -> NewUser<'_> {
    NewUser {
        gh_id: NEXT_GH_ID.fetch_add(1, Ordering::SeqCst) as i32,
        gh_login: login,
        name: None,
        gh_avatar: None,
        gh_access_token: Cow::Borrowed("some random token"),
    }
}

fn new_team(login: &str) -> NewTeam<'_> {
    NewTeam {
        github_id: NEXT_GH_ID.fetch_add(1, Ordering::SeqCst) as i32,
        login,
        name: None,
        avatar: None,
    }
}

fn add_team_to_crate(t: &Team, krate: &Crate, u: &User, conn: &PgConnection) -> QueryResult<()> {
    let crate_owner = CrateOwner {
        crate_id: krate.id,
        owner_id: t.id,
        created_by: u.id,
        owner_kind: 1, // Team owner kind is 1 according to owner.rs
        email_notifications: true,
    };

    diesel::insert_into(crate_owners::table)
        .values(&crate_owner)
        .on_conflict(crate_owners::table.primary_key())
        .do_update()
        .set(crate_owners::deleted.eq(false))
        .execute(conn)?;

    Ok(())
}

fn new_dependency(conn: &PgConnection, version: &Version, krate: &Crate) -> Dependency {
    use cargo_registry::schema::dependencies::dsl::*;
    use diesel::insert_into;

    insert_into(dependencies)
        .values((
            version_id.eq(version.id),
            crate_id.eq(krate.id),
            req.eq(">= 0"),
            optional.eq(false),
            default_features.eq(false),
            features.eq(Vec::<String>::new()),
        ))
        .get_result(conn)
        .unwrap()
}

fn new_category<'a>(category: &'a str, slug: &'a str, description: &'a str) -> NewCategory<'a> {
    NewCategory {
        category,
        slug,
        description,
    }
}

#[test]
fn multiple_live_references_to_the_same_connection_can_be_checked_out() {
    use std::ptr;

    let (app, _) = app();
    let conn1 = app.primary_database.get().unwrap();
    let conn2 = app.primary_database.get().unwrap();
    let conn1_ref: &PgConnection = &conn1;
    let conn2_ref: &PgConnection = &conn2;

    assert!(ptr::eq(conn1_ref, conn2_ref));
}
