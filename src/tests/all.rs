#![deny(warnings)]
#![allow(unknown_lints, proc_macro_derive_resolution_fallback)] // TODO: This can be removed after diesel-1.4

// Several test methods trip this clippy lint
#![cfg_attr(feature = "cargo-clippy", allow(cyclomatic_complexity))]

extern crate cargo_registry;
extern crate chrono;
extern crate conduit;
extern crate conduit_middleware;
extern crate conduit_test;
#[macro_use]
extern crate diesel;
extern crate dotenv;
extern crate flate2;
extern crate git2;
#[macro_use]
extern crate lazy_static;
extern crate s3;
extern crate semver;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate tar;
extern crate url;

use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use std::io::Read;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
use std::sync::Arc;

use cargo_registry::app::App;
use cargo_registry::middleware::current_user::AuthenticationSource;
use cargo_registry::Replica;
use chrono::Utc;
use conduit::{Method, Request};
use conduit_test::MockRequest;
use diesel::prelude::*;
use flate2::write::GzEncoder;
use flate2::Compression;

use cargo_registry::{models, schema, views};
use util::{Bad, RequestHelper, TestApp};

use models::{Crate, CrateOwner, Dependency, Team, User, Version};
use models::{NewCategory, NewTeam, NewUser, NewVersion};
use schema::*;
use views::krate_publish as u;
use views::{EncodableCrate, EncodableKeyword, EncodableVersion};

macro_rules! t {
    ($e:expr) => {
        match $e {
            Ok(e) => e,
            Err(m) => panic!("{} failed with: {}", stringify!($e), m),
        }
    };
}

macro_rules! ok_resp {
    ($e:expr) => {{
        let resp = t!($e);
        if !::ok_resp(&resp) {
            panic!("bad response: {:?}", resp.status);
        }
        resp
    }};
}

macro_rules! bad_resp {
    ($e:expr) => {{
        let mut resp = t!($e);
        match ::bad_resp(&mut resp) {
            None => panic!("ok response: {:?}", resp.status),
            Some(b) => b,
        }
    }};
}

mod badge;
mod builders;
mod categories;
mod category;
mod git;
mod keyword;
mod krate;
mod owners;
mod record;
mod schema_details;
mod server;
mod team;
mod token;
mod user;
mod util;
mod version;

#[derive(Deserialize, Debug)]
pub struct GoodCrate {
    #[serde(rename = "crate")]
    krate: EncodableCrate,
    warnings: Warnings,
}
#[derive(Deserialize)]
pub struct CrateList {
    crates: Vec<EncodableCrate>,
    meta: CrateMeta,
}
#[derive(Deserialize, Debug)]
struct Warnings {
    invalid_categories: Vec<String>,
    invalid_badges: Vec<String>,
}
#[derive(Deserialize)]
struct CrateMeta {
    total: i32,
}
#[derive(Deserialize)]
pub struct CrateResponse {
    #[serde(rename = "crate")]
    krate: EncodableCrate,
    versions: Vec<EncodableVersion>,
    keywords: Vec<EncodableKeyword>,
}
#[derive(Deserialize)]
struct OkBool {
    ok: bool,
}

fn app() -> (
    record::Bomb,
    Arc<App>,
    conduit_middleware::MiddlewareBuilder,
) {
    dotenv::dotenv().ok();

    let (proxy, bomb) = record::proxy();
    let uploader = cargo_registry::Uploader::S3 {
        bucket: s3::Bucket::new(
            String::from("alexcrichton-test"),
            None,
            std::env::var("S3_ACCESS_KEY").unwrap_or_default(),
            std::env::var("S3_SECRET_KEY").unwrap_or_default(),
            // When testing we route all API traffic over HTTP so we can
            // sniff/record it, but everywhere else we use https
            "http",
        ),
        proxy: Some(proxy),
        cdn: None,
    };

    let (app, handler) = simple_app(uploader);
    (bomb, app, handler)
}

fn simple_app(
    uploader: cargo_registry::Uploader,
) -> (Arc<App>, conduit_middleware::MiddlewareBuilder) {
    git::init();
    let config = cargo_registry::Config {
        uploader,
        session_key: "test this has to be over 32 bytes long".to_string(),
        git_repo_checkout: git::checkout(),
        gh_client_id: env::var("GH_CLIENT_ID").unwrap_or_default(),
        gh_client_secret: env::var("GH_CLIENT_SECRET").unwrap_or_default(),
        db_url: env("TEST_DATABASE_URL"),
        env: cargo_registry::Env::Test,
        max_upload_size: 3000,
        max_unpack_size: 2000,
        mirror: Replica::Primary,
        // When testing we route all API traffic over HTTP so we can
        // sniff/record it, but everywhere else we use https
        api_protocol: String::from("http"),
    };
    let app = App::new(&config);
    t!(t!(app.diesel_database.get()).begin_test_transaction());
    let app = Arc::new(app);
    let handler = cargo_registry::build_handler(Arc::clone(&app));
    (app, handler)
}

// Return the environment variable only if it has been defined
fn env(var: &str) -> String {
    match env::var(var) {
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

static NEXT_GH_ID: AtomicUsize = ATOMIC_USIZE_INIT;

fn new_user(login: &str) -> NewUser<'_> {
    NewUser {
        gh_id: NEXT_GH_ID.fetch_add(1, Ordering::SeqCst) as i32,
        gh_login: login,
        email: None,
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

fn add_team_to_crate(t: &Team, krate: &Crate, u: &User, conn: &PgConnection) -> CargoResult<()> {
    let crate_owner = CrateOwner {
        crate_id: krate.id,
        owner_id: t.id,
        created_by: u.id,
        owner_kind: 1, // Team owner kind is 1 according to owner.rs
    };

    diesel::insert_into(crate_owners::table)
        .values(&crate_owner)
        .on_conflict(crate_owners::table.primary_key())
        .do_update()
        .set(crate_owners::deleted.eq(false))
        .execute(conn)?;

    Ok(())
}

use cargo_registry::util::CargoResult;

fn new_version(crate_id: i32, num: &str, crate_size: Option<i32>) -> NewVersion {
    let num = semver::Version::parse(num).unwrap();
    NewVersion::new(crate_id, &num, &HashMap::new(), None, None, crate_size).unwrap()
}

fn krate(name: &str) -> Crate {
    static NEXT_CRATE_ID: AtomicUsize = ATOMIC_USIZE_INIT;

    Crate {
        id: NEXT_CRATE_ID.fetch_add(1, Ordering::SeqCst) as i32,
        name: name.to_string(),
        updated_at: Utc::now().naive_utc(),
        created_at: Utc::now().naive_utc(),
        downloads: 10,
        documentation: None,
        homepage: None,
        description: None,
        readme: None,
        readme_file: None,
        license: None,
        repository: None,
        max_upload_size: None,
    }
}

fn new_crate(name: &str, version: &str) -> u::NewCrate {
    u::NewCrate {
        name: u::CrateName(name.to_string()),
        vers: u::CrateVersion(semver::Version::parse(version).unwrap()),
        features: HashMap::new(),
        deps: Vec::new(),
        authors: vec!["foo".to_string()],
        description: Some("desc".to_string()),
        homepage: None,
        documentation: None,
        readme: None,
        readme_file: None,
        keywords: None,
        categories: None,
        license: Some("MIT".to_string()),
        license_file: None,
        repository: None,
        badges: None,
        links: None,
    }
}

fn sign_in_as(req: &mut Request, user: &User) {
    req.mut_extensions().insert(user.clone());
    req.mut_extensions()
        .insert(AuthenticationSource::SessionCookie);
}

fn sign_in(req: &mut Request, app: &App) -> User {
    let conn = app.diesel_database.get().unwrap();
    let user = new_user("foo").create_or_update(&conn).unwrap();
    sign_in_as(req, &user);
    user
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

fn logout(req: &mut Request) {
    req.mut_extensions().pop::<User>();
}

fn new_req(krate: &str, version: &str) -> MockRequest {
    new_req_full(::krate(krate), version, Vec::new())
}

fn new_req_with_documentation(krate: &str, version: &str, documentation: &str) -> MockRequest {
    let mut krate = ::krate(krate);
    krate.documentation = Some(documentation.into());
    new_req_full(krate, version, Vec::new())
}

fn new_req_full(krate: Crate, version: &str, deps: Vec<u::CrateDependency>) -> MockRequest {
    let mut req = req(Method::Put, "/api/v1/crates/new");
    req.with_body(&new_req_body(
        krate,
        version,
        deps,
        Vec::new(),
        Vec::new(),
        HashMap::new(),
    ));
    req
}

fn new_req_with_keywords(krate: Crate, version: &str, kws: Vec<String>) -> MockRequest {
    let mut req = req(Method::Put, "/api/v1/crates/new");
    req.with_body(&new_req_body(
        krate,
        version,
        Vec::new(),
        kws,
        Vec::new(),
        HashMap::new(),
    ));
    req
}

fn new_req_with_categories(krate: Crate, version: &str, cats: Vec<String>) -> MockRequest {
    let mut req = req(Method::Put, "/api/v1/crates/new");
    req.with_body(&new_req_body(
        krate,
        version,
        Vec::new(),
        Vec::new(),
        cats,
        HashMap::new(),
    ));
    req
}

fn new_req_with_badges(
    krate: Crate,
    version: &str,
    badges: HashMap<String, HashMap<String, String>>,
) -> MockRequest {
    let mut req = req(Method::Put, "/api/v1/crates/new");
    req.with_body(&new_req_body(
        krate,
        version,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        badges,
    ));
    req
}

fn new_req_body_version_2(krate: Crate) -> Vec<u8> {
    new_req_body(
        krate,
        "2.0.0",
        Vec::new(),
        Vec::new(),
        Vec::new(),
        HashMap::new(),
    )
}

fn new_req_body(
    krate: Crate,
    version: &str,
    deps: Vec<u::CrateDependency>,
    kws: Vec<String>,
    cats: Vec<String>,
    badges: HashMap<String, HashMap<String, String>>,
) -> Vec<u8> {
    let kws = kws.into_iter().map(u::Keyword).collect();
    let cats = cats.into_iter().map(u::Category).collect();

    new_crate_to_body(
        &u::NewCrate {
            name: u::CrateName(krate.name),
            vers: u::CrateVersion(semver::Version::parse(version).unwrap()),
            features: HashMap::new(),
            deps,
            authors: vec!["foo".to_string()],
            description: Some("description".to_string()),
            homepage: krate.homepage,
            documentation: krate.documentation,
            readme: krate.readme,
            readme_file: krate.readme_file,
            keywords: Some(u::KeywordList(kws)),
            categories: Some(u::CategoryList(cats)),
            license: Some("MIT".to_string()),
            license_file: None,
            repository: krate.repository,
            badges: Some(badges),
            links: None,
        },
        &[],
    )
}

fn new_crate_to_body(new_crate: &u::NewCrate, files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut slices = files.iter().map(|p| p.1).collect::<Vec<_>>();
    let mut files = files
        .iter()
        .zip(&mut slices)
        .map(|(&(name, _), data)| {
            let len = data.len() as u64;
            (name, data as &mut Read, len)
        })
        .collect::<Vec<_>>();
    new_crate_to_body_with_io(new_crate, &mut files)
}

fn new_crate_to_body_with_io(
    new_crate: &u::NewCrate,
    files: &mut [(&str, &mut Read, u64)],
) -> Vec<u8> {
    let mut tarball = Vec::new();
    {
        let mut ar = tar::Builder::new(GzEncoder::new(&mut tarball, Compression::default()));
        for &mut (name, ref mut data, size) in files {
            let mut header = tar::Header::new_gnu();
            t!(header.set_path(name));
            header.set_size(size);
            header.set_cksum();
            t!(ar.append(&header, data));
        }
        t!(ar.finish());
    }
    new_crate_to_body_with_tarball(new_crate, &tarball)
}

fn new_crate_to_body_with_tarball(new_crate: &u::NewCrate, tarball: &[u8]) -> Vec<u8> {
    let json = serde_json::to_string(&new_crate).unwrap();
    let mut body = Vec::new();
    body.extend(
        [
            json.len() as u8,
            (json.len() >> 8) as u8,
            (json.len() >> 16) as u8,
            (json.len() >> 24) as u8,
        ]
        .iter()
        .cloned(),
    );
    body.extend(json.as_bytes().iter().cloned());
    body.extend(&[
        tarball.len() as u8,
        (tarball.len() >> 8) as u8,
        (tarball.len() >> 16) as u8,
        (tarball.len() >> 24) as u8,
    ]);
    body.extend(tarball);
    body
}
