#![deny(warnings)]

extern crate diesel;
extern crate bufstream;
extern crate cargo_registry;
extern crate conduit;
extern crate conduit_middleware;
extern crate conduit_test;
extern crate curl;
extern crate dotenv;
extern crate git2;
extern crate postgres;
extern crate rustc_serialize;
extern crate semver;
extern crate time;
extern crate url;
extern crate s3;

use rustc_serialize::json::{self, Json};
use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};
use std::sync::Arc;

use cargo_registry::app::App;
use cargo_registry::category::NewCategory;
use cargo_registry::db::{self, RequestTransaction};
use cargo_registry::dependency::{Kind, NewDependency};
use cargo_registry::keyword::Keyword;
use cargo_registry::krate::NewCrate;
use cargo_registry::upload as u;
use cargo_registry::user::NewUser;
use cargo_registry::version::NewVersion;
use cargo_registry::{User, Crate, Version, Dependency, Category, Model, Replica};
use conduit::{Request, Method};
use conduit_test::MockRequest;
use diesel::pg::PgConnection;
use diesel::prelude::*;

macro_rules! t {
    ($e:expr) => (
        match $e {
            Ok(e) => e,
            Err(m) => panic!("{} failed with: {}", stringify!($e), m),
        }
    )
}

macro_rules! t_resp { ($e:expr) => (t!($e)) }

macro_rules! ok_resp {
    ($e:expr) => ({
        let resp = t_resp!($e);
        if !::ok_resp(&resp) { panic!("bad response: {:?}", resp.status); }
        resp
    })
}

macro_rules! bad_resp {
    ($e:expr) => ({
        let mut resp = t_resp!($e);
        match ::bad_resp(&mut resp) {
            None => panic!("ok response: {:?}", resp.status),
            Some(b) => b,
        }
    })
}

#[derive(RustcDecodable, Debug)]
struct Error {
    detail: String,
}
#[derive(RustcDecodable)]
struct Bad {
    errors: Vec<Error>,
}

mod badge;
mod category;
mod git;
mod keyword;
mod krate;
mod record;
mod team;
mod user;
mod version;

fn app() -> (record::Bomb, Arc<App>, conduit_middleware::MiddlewareBuilder) {
    dotenv::dotenv().ok();
    git::init();

    let (proxy, bomb) = record::proxy();

    // When testing we route all API traffic over HTTP so we can
    // sniff/record it, but everywhere else we use https
    let api_protocol = String::from("http");

    let uploader = cargo_registry::Uploader::S3 {
        bucket: s3::Bucket::new(
            String::from("alexcrichton-test"),
            None,
            String::new(),
            String::new(),
            &api_protocol,
        ),
        proxy: Some(proxy),
    };

    let config = cargo_registry::Config {
        uploader: uploader,
        session_key: "test".to_string(),
        git_repo_checkout: git::checkout(),
        gh_client_id: env::var("GH_CLIENT_ID").unwrap_or(String::new()),
        gh_client_secret: env::var("GH_CLIENT_SECRET").unwrap_or(String::new()),
        db_url: env("TEST_DATABASE_URL"),
        env: cargo_registry::Env::Test,
        max_upload_size: 1000,
        mirror: Replica::Primary,
        api_protocol: api_protocol,
    };
    let app = App::new(&config);
    t!(t!(app.diesel_database.get()).begin_test_transaction());
    let app = Arc::new(app);
    let middleware = cargo_registry::middleware(app.clone());
    return (bomb, app, middleware);
}

fn env(s: &str) -> String {
    match env::var(s).ok() {
        Some(s) => s,
        None => panic!("must have `{}` defined", s),
    }
}

fn req(app: Arc<App>, method: conduit::Method, path: &str) -> MockRequest {
    let mut req = MockRequest::new(method, path);
    req.mut_extensions().insert(db::Transaction::new(app));
    return req;
}

fn ok_resp(r: &conduit::Response) -> bool {
    r.status.0 == 200
}

fn bad_resp(r: &mut conduit::Response) -> Option<Bad> {
    let bad = json::<Bad>(r);
    if bad.errors.len() == 0 {
        return None;
    }
    Some(bad)
}

fn json<T: rustc_serialize::Decodable>(r: &mut conduit::Response) -> T {
    let mut data = Vec::new();
    r.body.write_body(&mut data).unwrap();
    let s = std::str::from_utf8(&data).unwrap();
    let j = match Json::from_str(s) {
        Ok(t) => t,
        Err(e) => panic!("failed to decode: {:?}\n{}", e, s),
    };
    let j = fixup(j);
    let s = j.to_string();
    return match json::decode(&s) {
        Ok(t) => t,
        Err(e) => panic!("failed to decode: {:?}\n{}", e, s),
    };


    fn fixup(json: Json) -> Json {
        match json {
            Json::Object(object) => {
                Json::Object(
                    object
                        .into_iter()
                        .map(|(k, v)| {
                            let k = if k == "crate" { "krate".to_string() } else { k };
                            (k, fixup(v))
                        })
                        .collect(),
                )
            }
            Json::Array(list) => Json::Array(list.into_iter().map(fixup).collect()),
            j => j,
        }
    }
}

static NEXT_ID: AtomicUsize = ATOMIC_USIZE_INIT;

fn new_user(login: &str) -> NewUser {
    NewUser {
        gh_id: NEXT_ID.fetch_add(1, Ordering::SeqCst) as i32,
        gh_login: login,
        email: None,
        name: None,
        gh_avatar: None,
        gh_access_token: Cow::Borrowed("some random token"),
    }
}

fn user(login: &str) -> User {
    User {
        id: NEXT_ID.fetch_add(1, Ordering::SeqCst) as i32,
        gh_id: NEXT_ID.fetch_add(1, Ordering::SeqCst) as i32,
        gh_login: login.to_string(),
        email: None,
        name: None,
        gh_avatar: None,
        gh_access_token: "some random token".into(),
        api_token: "some random token".into(),
    }
}

use cargo_registry::util::CargoResult;

struct VersionBuilder<'a> {
    version: semver::Version,
    license: Option<&'a str>,
    license_file: Option<&'a str>,
    features: HashMap<String, Vec<String>>,
}

impl<'a> VersionBuilder<'a> {
    fn new(version: &str) -> VersionBuilder {
        let version = semver::Version::parse(version).unwrap_or_else(|e| {
            panic!("The version {} is not valid: {}", version, e);
        });

        VersionBuilder {
            version: version,
            license: None,
            license_file: None,
            features: HashMap::new(),
        }
    }

    fn license(mut self, license: Option<&'a str>) -> Self {
        self.license = license;
        self
    }

    fn build(self, connection: &PgConnection, crate_id: i32) -> CargoResult<Version> {
        let license = match self.license {
            Some(license) => Some(license.to_owned()),
            None => None,
        };

        NewVersion::new(
            crate_id,
            &self.version,
            &self.features,
            license,
            self.license_file,
        )?
            .save(connection, &[])
    }
}

struct CrateBuilder<'a> {
    owner_id: i32,
    krate: NewCrate<'a>,
    downloads: Option<i32>,
    versions: Vec<VersionBuilder<'a>>,
    keywords: Vec<&'a str>,
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
            versions: Vec::new(),
            keywords: Vec::new(),
        }
    }

    fn description(mut self, description: &'a str) -> Self {
        self.krate.description = Some(description);
        self
    }

    fn documentation(mut self, documentation: &'a str) -> Self {
        self.krate.documentation = Some(documentation);
        self
    }

    fn homepage(mut self, homepage: &'a str) -> Self {
        self.krate.homepage = Some(homepage);
        self
    }

    fn readme(mut self, readme: &'a str) -> Self {
        self.krate.readme = Some(readme);
        self
    }

    fn max_upload_size(mut self, max_upload_size: i32) -> Self {
        self.krate.max_upload_size = Some(max_upload_size);
        self
    }

    fn downloads(mut self, downloads: i32) -> Self {
        self.downloads = Some(downloads);
        self
    }

    fn version(mut self, version: VersionBuilder<'a>) -> Self {
        self.versions.push(version);
        self
    }

    fn keyword(mut self, keyword: &'a str) -> Self {
        self.keywords.push(keyword);
        self
    }

    fn build(mut self, connection: &PgConnection) -> CargoResult<Crate> {
        use diesel::update;

        let mut krate = self.krate.create_or_update(connection, None, self.owner_id)?;

        // Since we are using `NewCrate`, we can't set all the
        // crate properties in a single DB call.
        if let Some(downloads) = self.downloads {
            krate.downloads = downloads;
            update(&krate).set(&krate).execute(connection)?;
        }

        if self.versions.is_empty() {
            self.versions.push(VersionBuilder::new("0.99.0"));
        }

        for version in self.versions {
            version.build(&connection, krate.id)?;
        }

        if !self.keywords.is_empty() {
            Keyword::update_crate(connection, &krate, &self.keywords)?;
        }

        Ok(krate)
    }

    fn expect_build(self, connection: &PgConnection) -> Crate {
        let name = self.krate.name;
        self.build(connection).unwrap_or_else(|e| {
            panic!("Unable to create crate {}: {:?}", name, e);
        })
    }
}

fn new_version(crate_id: i32, num: &str) -> NewVersion {
    let num = semver::Version::parse(num).unwrap();
    NewVersion::new(crate_id, &num, &HashMap::new(), None, None).unwrap()
}

fn krate(name: &str) -> Crate {
    cargo_registry::krate::Crate {
        id: NEXT_ID.fetch_add(1, Ordering::SeqCst) as i32,
        name: name.to_string(),
        updated_at: time::now().to_timespec(),
        created_at: time::now().to_timespec(),
        downloads: 10,
        documentation: None,
        homepage: None,
        description: None,
        readme: None,
        license: None,
        repository: None,
        max_upload_size: None,
    }
}

fn mock_user(req: &mut Request, u: User) -> User {
    let u = User::find_or_insert(
        req.tx().unwrap(),
        u.gh_id,
        &u.gh_login,
        u.email.as_ref().map(|s| &s[..]),
        u.name.as_ref().map(|s| &s[..]),
        u.gh_avatar.as_ref().map(|s| &s[..]),
        &u.gh_access_token,
    ).unwrap();
    sign_in_as(req, &u);
    return u;
}

fn sign_in_as(req: &mut Request, user: &User) {
    req.mut_extensions().insert(user.clone());
}

fn sign_in(req: &mut Request, app: &App) {
    let conn = app.diesel_database.get().unwrap();
    let user = ::new_user("foo").create_or_update(&conn).unwrap();
    sign_in_as(req, &user);
}

fn mock_crate(req: &mut Request, krate: Crate) -> (Crate, Version) {
    mock_crate_vers(req, krate, &semver::Version::parse("1.0.0").unwrap())
}

fn mock_crate_vers(req: &mut Request, krate: Crate, v: &semver::Version) -> (Crate, Version) {
    let user = req.extensions().find::<User>().unwrap();
    let mut krate = Crate::find_or_insert(
        req.tx().unwrap(),
        &krate.name,
        user.id,
        &krate.description,
        &krate.homepage,
        &krate.documentation,
        &krate.readme,
        &krate.repository,
        &krate.license,
        &None,
        krate.max_upload_size,
    ).unwrap();
    let v = krate.add_version(req.tx().unwrap(), v, &HashMap::new(), &[]);
    (krate, v.unwrap())
}

fn new_dependency(conn: &PgConnection, version: &Version, krate: &Crate) -> Dependency {
    use diesel::insert;
    use cargo_registry::schema::dependencies;

    let dep = NewDependency {
        version_id: version.id,
        crate_id: krate.id,
        req: ">= 0".into(),
        optional: false,
        ..Default::default()
    };
    insert(&dep)
        .into(dependencies::table)
        .get_result(conn)
        .unwrap()
}

fn mock_dep(
    req: &mut Request,
    version: &Version,
    krate: &Crate,
    target: Option<&str>,
) -> Dependency {
    Dependency::insert(
        req.tx().unwrap(),
        version.id,
        krate.id,
        &semver::VersionReq::parse(">= 0").unwrap(),
        Kind::Normal,
        false,
        true,
        &[],
        &target.map(|s| s.to_string()),
    ).unwrap()
}

fn new_category<'a>(category: &'a str, slug: &'a str) -> NewCategory<'a> {
    NewCategory {
        category: category,
        slug: slug,
        ..NewCategory::default()
    }
}

fn mock_category(req: &mut Request, name: &str, slug: &str) -> Category {
    let conn = req.tx().unwrap();
    let stmt = conn.prepare(
        " \
         INSERT INTO categories (category, slug) \
         VALUES ($1, $2) \
         RETURNING *",
    ).unwrap();
    let rows = stmt.query(&[&name, &slug]).unwrap();
    Model::from_row(&rows.iter().next().unwrap())
}

fn logout(req: &mut Request) {
    req.mut_extensions().pop::<User>();
}

fn request_with_user_and_mock_crate(app: &Arc<App>, user: NewUser, krate: &str) -> MockRequest {
    let mut req = new_req(app.clone(), krate, "1.0.0");
    {
        let conn = app.diesel_database.get().unwrap();
        let user = user.create_or_update(&conn).unwrap();
        sign_in_as(&mut req, &user);
        ::CrateBuilder::new(krate, user.id).expect_build(&conn);
    }
    req
}

fn new_req(app: Arc<App>, krate: &str, version: &str) -> MockRequest {
    new_req_full(app, ::krate(krate), version, Vec::new())
}

fn new_req_full(
    app: Arc<App>,
    krate: Crate,
    version: &str,
    deps: Vec<u::CrateDependency>,
) -> MockRequest {
    let mut req = ::req(app, Method::Put, "/api/v1/crates/new");
    req.with_body(&new_req_body(
        krate,
        version,
        deps,
        Vec::new(),
        Vec::new(),
        HashMap::new(),
    ));
    return req;
}

fn new_req_with_keywords(
    app: Arc<App>,
    krate: Crate,
    version: &str,
    kws: Vec<String>,
) -> MockRequest {
    let mut req = ::req(app, Method::Put, "/api/v1/crates/new");
    req.with_body(&new_req_body(
        krate,
        version,
        Vec::new(),
        kws,
        Vec::new(),
        HashMap::new(),
    ));
    return req;
}

fn new_req_with_categories(
    app: Arc<App>,
    krate: Crate,
    version: &str,
    cats: Vec<String>,
) -> MockRequest {
    let mut req = ::req(app, Method::Put, "/api/v1/crates/new");
    req.with_body(&new_req_body(
        krate,
        version,
        Vec::new(),
        Vec::new(),
        cats,
        HashMap::new(),
    ));
    return req;
}

fn new_req_with_badges(
    app: Arc<App>,
    krate: Crate,
    version: &str,
    badges: HashMap<String, HashMap<String, String>>,
) -> MockRequest {
    let mut req = ::req(app, Method::Put, "/api/v1/crates/new");
    req.with_body(&new_req_body(
        krate,
        version,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        badges,
    ));
    return req;
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
            deps: deps,
            authors: vec!["foo".to_string()],
            description: Some("description".to_string()),
            homepage: krate.homepage,
            documentation: krate.documentation,
            readme: krate.readme,
            keywords: Some(u::KeywordList(kws)),
            categories: Some(u::CategoryList(cats)),
            license: Some("MIT".to_string()),
            license_file: None,
            repository: krate.repository,
            badges: Some(badges),
        },
        &[],
    )
}

fn new_crate_to_body(new_crate: &u::NewCrate, krate: &[u8]) -> Vec<u8> {
    let json = json::encode(&new_crate).unwrap();
    let mut body = Vec::new();
    body.extend(
        [
            (json.len() >> 0) as u8,
            (json.len() >> 8) as u8,
            (json.len() >> 16) as u8,
            (json.len() >> 24) as u8,
        ].iter()
            .cloned(),
    );
    body.extend(json.as_bytes().iter().cloned());
    body.extend(
        &[
            (krate.len() >> 0) as u8,
            (krate.len() >> 8) as u8,
            (krate.len() >> 16) as u8,
            (krate.len() >> 24) as u8,
        ],
    );
    body.extend(krate);
    body
}
