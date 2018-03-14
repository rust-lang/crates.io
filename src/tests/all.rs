#![deny(warnings)]

extern crate cargo_registry;
extern crate chrono;
extern crate conduit;
extern crate conduit_middleware;
extern crate conduit_test;
extern crate curl;
#[macro_use]
extern crate diesel;
extern crate dotenv;
extern crate flate2;
extern crate git2;
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
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

use cargo_registry::app::App;
use cargo_registry::user::AuthenticationSource;
use cargo_registry::Replica;
use chrono::Utc;
use conduit::{Method, Request};
use conduit_test::MockRequest;
use diesel::prelude::*;
use flate2::Compression;
use flate2::write::GzEncoder;

pub use cargo_registry::{models, schema, views};

use views::EncodableCrate;
use views::krate_publish as u;
use models::{Crate, CrateDownload, CrateOwner, Dependency, Keyword, Team, User, Version};
use models::{NewCategory, NewCrate, NewTeam, NewUser, NewVersion};
use schema::*;

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

#[derive(Deserialize, Debug)]
struct Error {
    detail: String,
}
#[derive(Deserialize)]
struct Bad {
    errors: Vec<Error>,
}

mod badge;
mod categories;
mod category;
mod git;
mod keyword;
mod krate;
mod owners;
mod record;
mod schema_details;
mod team;
mod token;
mod user;
mod version;

#[derive(Deserialize, Debug)]
struct GoodCrate {
    #[serde(rename = "crate")]
    krate: EncodableCrate,
    warnings: Warnings,
}
#[derive(Deserialize)]
struct CrateList {
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

fn app() -> (
    record::Bomb,
    Arc<App>,
    conduit_middleware::MiddlewareBuilder,
) {
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
            std::env::var("S3_ACCESS_KEY").unwrap_or_default(),
            std::env::var("S3_SECRET_KEY").unwrap_or_default(),
            &api_protocol,
        ),
        proxy: Some(proxy),
        cdn: None,
    };

    let config = cargo_registry::Config {
        uploader: uploader,
        session_key: "test this has to be over 32 bytes long".to_string(),
        git_repo_checkout: git::checkout(),
        gh_client_id: env::var("GH_CLIENT_ID").unwrap_or_default(),
        gh_client_secret: env::var("GH_CLIENT_SECRET").unwrap_or_default(),
        db_url: env("TEST_DATABASE_URL"),
        env: cargo_registry::Env::Test,
        max_upload_size: 1000,
        max_unpack_size: 2000,
        mirror: Replica::Primary,
        api_protocol: api_protocol,
    };
    let app = App::new(&config);
    t!(t!(app.diesel_database.get()).begin_test_transaction());
    let app = Arc::new(app);
    let middleware = cargo_registry::middleware(Arc::clone(&app));
    (bomb, app, middleware)
}

// Return the environment variable only if it has been defined
fn env(s: &str) -> String {
    // Handles both the `None` and empty string cases e.g. VAR=
    // by converting `None` to an empty string
    let env_result = env::var(s).ok().unwrap_or_default();

    if env_result == "" {
        panic!("must have `{}` defined", s);
    }

    env_result
}

fn req(_: Arc<App>, method: conduit::Method, path: &str) -> MockRequest {
    MockRequest::new(method, path)
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
    }
}

fn new_team(login: &str) -> NewTeam {
    NewTeam {
        github_id: NEXT_ID.fetch_add(1, Ordering::SeqCst) as i32,
        login: login,
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

struct VersionBuilder<'a> {
    num: semver::Version,
    license: Option<&'a str>,
    license_file: Option<&'a str>,
    features: HashMap<String, Vec<String>>,
    dependencies: Vec<(i32, Option<&'static str>)>,
}

impl<'a> VersionBuilder<'a> {
    fn new(num: &str) -> Self {
        let num = semver::Version::parse(num).unwrap_or_else(|e| {
            panic!("The version {} is not valid: {}", num, e);
        });

        VersionBuilder {
            num,
            license: None,
            license_file: None,
            features: HashMap::new(),
            dependencies: Vec::new(),
        }
    }

    fn license(mut self, license: Option<&'a str>) -> Self {
        self.license = license;
        self
    }

    fn dependency(mut self, dependency: &Crate, target: Option<&'static str>) -> Self {
        self.dependencies.push((dependency.id, target));
        self
    }

    fn build(self, crate_id: i32, connection: &PgConnection) -> CargoResult<Version> {
        use diesel::insert_into;

        let license = match self.license {
            Some(license) => Some(license.to_owned()),
            None => None,
        };

        let vers = NewVersion::new(
            crate_id,
            &self.num,
            &self.features,
            license,
            self.license_file,
        )?.save(connection, &[])?;

        let new_deps = self.dependencies
            .into_iter()
            .map(|(crate_id, target)| {
                (
                    dependencies::version_id.eq(vers.id),
                    dependencies::req.eq(">= 0"),
                    dependencies::crate_id.eq(crate_id),
                    dependencies::target.eq(target),
                    dependencies::optional.eq(false),
                    dependencies::default_features.eq(false),
                    dependencies::features.eq(Vec::<String>::new()),
                )
            })
            .collect::<Vec<_>>();
        insert_into(dependencies::table)
            .values(&new_deps)
            .execute(connection)?;

        Ok(vers)
    }
}

impl<'a> From<&'a str> for VersionBuilder<'a> {
    fn from(num: &'a str) -> Self {
        VersionBuilder::new(num)
    }
}

struct CrateBuilder<'a> {
    owner_id: i32,
    krate: NewCrate<'a>,
    downloads: Option<i32>,
    recent_downloads: Option<i32>,
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
            recent_downloads: None,
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

    fn recent_downloads(mut self, recent_downloads: i32) -> Self {
        self.recent_downloads = Some(recent_downloads);
        self
    }

    fn version<T: Into<VersionBuilder<'a>>>(mut self, version: T) -> Self {
        self.versions.push(version.into());
        self
    }

    fn keyword(mut self, keyword: &'a str) -> Self {
        self.keywords.push(keyword);
        self
    }

    fn build(mut self, connection: &PgConnection) -> CargoResult<Crate> {
        use diesel::{insert_into, update};

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
        }

        if self.versions.is_empty() {
            self.versions.push(VersionBuilder::new("0.99.0"));
        }

        for version_builder in self.versions {
            version_builder.build(krate.id, connection)?;
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
    Crate {
        id: NEXT_ID.fetch_add(1, Ordering::SeqCst) as i32,
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

fn sign_in_as(req: &mut Request, user: &User) {
    req.mut_extensions().insert(user.clone());
    req.mut_extensions()
        .insert(AuthenticationSource::SessionCookie);
}

fn sign_in(req: &mut Request, app: &App) -> User {
    let conn = app.diesel_database.get().unwrap();
    let user = ::new_user("foo").create_or_update(&conn).unwrap();
    sign_in_as(req, &user);
    user
}

fn new_dependency(conn: &PgConnection, version: &Version, krate: &Crate) -> Dependency {
    use diesel::insert_into;
    use cargo_registry::schema::dependencies::dsl::*;

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

fn new_category<'a>(category: &'a str, slug: &'a str) -> NewCategory<'a> {
    NewCategory {
        category: category,
        slug: slug,
    }
}

fn logout(req: &mut Request) {
    req.mut_extensions().pop::<User>();
}

fn request_with_user_and_mock_crate(app: &Arc<App>, user: &NewUser, krate: &str) -> MockRequest {
    let mut req = new_req(Arc::clone(app), krate, "1.0.0");
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

fn new_req_with_documentation(
    app: Arc<App>,
    krate: &str,
    version: &str,
    documentation: &str,
) -> MockRequest {
    let mut krate = ::krate(krate);
    krate.documentation = Some(documentation.into());
    new_req_full(app, krate, version, Vec::new())
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
    req
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
    req
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
    req
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
            deps: deps,
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
    let json = serde_json::to_string(&new_crate).unwrap();
    let mut body = Vec::new();
    body.extend(
        [
            json.len() as u8,
            (json.len() >> 8) as u8,
            (json.len() >> 16) as u8,
            (json.len() >> 24) as u8,
        ].iter()
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
