#![deny(warnings)]
#![allow(unknown_lints, proc_macro_derive_resolution_fallback)] // This can be removed after diesel-1.4

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
use conduit::{Handler, Method, Request};
use conduit_test::MockRequest;
use diesel::prelude::*;
use flate2::write::GzEncoder;
use flate2::Compression;

pub use cargo_registry::{models, schema, views};

use models::{Crate, CrateDownload, CrateOwner, Dependency, Keyword, Team, User, Version};
use models::{NewCategory, NewCrate, NewTeam, NewUser, NewVersion};
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

macro_rules! t_resp {
    ($e:expr) => {
        t!($e)
    };
}

macro_rules! ok_resp {
    ($e:expr) => {{
        let resp = t_resp!($e);
        if !::ok_resp(&resp) {
            panic!("bad response: {:?}", resp.status);
        }
        resp
    }};
}

macro_rules! bad_resp {
    ($e:expr) => {{
        let mut resp = t_resp!($e);
        match ::bad_resp(&mut resp) {
            None => panic!("ok response: {:?}", resp.status),
            Some(b) => b,
        }
    }};
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
        max_upload_size: 3000,
        max_unpack_size: 2000,
        mirror: Replica::Primary,
        api_protocol: api_protocol,
    };
    let app = App::new(&config);
    t!(t!(app.diesel_database.get()).begin_test_transaction());
    let app = Arc::new(app);
    let handler = cargo_registry::build_handler(Arc::clone(&app));
    (bomb, app, handler)
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

fn new_user(login: &str) -> NewUser<'_> {
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

fn new_team(login: &str) -> NewTeam<'_> {
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
    yanked: bool,
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
            yanked: false,
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

    fn yanked(self, yanked: bool) -> Self {
        Self { yanked, ..self }
    }

    fn build(self, crate_id: i32, connection: &PgConnection) -> CargoResult<Version> {
        use diesel::{insert_into, update};

        let license = match self.license {
            Some(license) => Some(license.to_owned()),
            None => None,
        };

        let mut vers = NewVersion::new(
            crate_id,
            &self.num,
            &self.features,
            license,
            self.license_file,
            None,
        )?.save(connection, &[])?;

        if self.yanked {
            vers = update(&vers)
                .set(versions::yanked.eq(true))
                .get_result(connection)?;
        }

        let new_deps = self
            .dependencies
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
            }).collect::<Vec<_>>();
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
    fn new(name: &str, owner_id: i32) -> CrateBuilder<'_> {
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
        use diesel::{insert_into, select, update};

        let mut krate = self
            .krate
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

fn new_version(crate_id: i32, num: &str, crate_size: Option<i32>) -> NewVersion {
    let num = semver::Version::parse(num).unwrap();
    NewVersion::new(crate_id, &num, &HashMap::new(), None, None, crate_size).unwrap()
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
        )).get_result(conn)
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

fn request_with_user_and_mock_crate(
    app: &Arc<App>,
    user: &NewUser<'_>,
    krate: &str,
) -> MockRequest {
    let mut req = new_req(Arc::clone(app), krate, "1.0.0");
    {
        let conn = app.diesel_database.get().unwrap();
        let user = user.create_or_update(&conn).unwrap();
        sign_in_as(&mut req, &user);
        CrateBuilder::new(krate, user.id).expect_build(&conn);
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
    let mut req = req(app, Method::Put, "/api/v1/crates/new");
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
    let mut req = req(app, Method::Put, "/api/v1/crates/new");
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
    let mut req = req(app, Method::Put, "/api/v1/crates/new");
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
    let mut req = req(app, Method::Put, "/api/v1/crates/new");
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
        }).collect::<Vec<_>>();
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

/// A struct representing a browser session to be used for the duration of a test.
/// Has useful methods for making common HTTP requests.
pub struct BrowserSession {
    app: Arc<App>,
    // The bomb needs to be held in scope until the end of the test.
    _bomb: record::Bomb,
    middle: conduit_middleware::MiddlewareBuilder,
    request: MockRequest,
    _user: User,
}

impl BrowserSession {
    /// Create a browser session logged in with an arbitrary user.
    pub fn logged_in() -> Self {
        let (bomb, app, middle) = app();

        let mut request = MockRequest::new(Method::Get, "/");
        let user = {
            let conn = app.diesel_database.get().unwrap();
            let user = new_user("foo").create_or_update(&conn).unwrap();
            request.mut_extensions().insert(user.clone());
            request.mut_extensions()
                .insert(AuthenticationSource::SessionCookie);

            user
        };

        BrowserSession {
            app, _bomb: bomb, middle, request, _user: user,
        }
    }

    /// Create a browser session logged in with the given user.
    // pub fn logged_in_as(_user: &User) -> Self {
    //     unimplemented!();
    // }

    /// For internal use only: make the current request
    fn make_request(&mut self) -> conduit::Response {
        ok_resp!(self.middle.call(&mut self.request))
    }

    /// Log out the currently logged in user.
    pub fn logout(&mut self) {
        logout(&mut self.request);
    }

    /// Using the same session, log in as a different user.
    pub fn log_in_as(&mut self, user: &User) {
        sign_in_as(&mut self.request, user);
    }

    /// Request the JSON used for a crate's page.
    pub fn show_crate(&mut self, krate_name: &str) -> CrateResponse {
        self.request.with_method(Method::Get).with_path(&format!("/api/v1/crates/{}", krate_name));
        let mut response = self.make_request();
        json(&mut response)
    }

    /// Add a user as an owner for a crate.
    pub fn add_owner(&mut self, krate_name: &str, user: &User) {
        let body = format!("{{\"users\":[\"{}\"]}}", user.gh_login);
        self.request
            .with_path(&format!("/api/v1/crates/{}/owners", krate_name))
            .with_method(Method::Put)
            .with_body(body.as_bytes());

        let mut response = self.make_request();

        #[derive(Deserialize)]
        struct O {
            ok: bool,
        }
        assert!(json::<O>(&mut response).ok);
    }

    /// As the currently logged in user, accept an invitation to become an owner of the named
    /// crate.
    pub fn accept_ownership_invitation(&mut self, krate_name: &str) {
        use views::InvitationResponse;

        let krate_id = {
            let conn = self.app.diesel_database.get().unwrap();
            Crate::by_name(krate_name)
                .first::<Crate>(&*conn)
                .unwrap()
                .id
        };

        let body = json!({
            "crate_owner_invite": {
                "invited_by_username": "",
                "crate_name": krate_name,
                "crate_id": krate_id,
                "created_at": "",
                "accepted": true
            }
        });

        self.request
            .with_path(&format!("/api/v1/me/crate_owner_invitations/{}", krate_id))
            .with_method(Method::Put)
            .with_body(body.to_string().as_bytes());

        let mut response = self.make_request();

        #[derive(Deserialize)]
        struct CrateOwnerInvitation {
            crate_owner_invitation: InvitationResponse,
        }

        let crate_owner_invite = ::json::<CrateOwnerInvitation>(&mut response);
        assert!(crate_owner_invite.crate_owner_invitation.accepted);
        assert_eq!(crate_owner_invite.crate_owner_invitation.crate_id, krate_id);
    }

    /// Get the crates owned by the specified user.
    pub fn crates_owned_by(&mut self, user: &User) -> CrateList {
        let query = format!("user_id={}", user.id);
        self.request
            .with_path("/api/v1/crates")
            .with_method(Method::Get)
            .with_query(&query);

        let mut response = self.make_request();

        json::<CrateList>(&mut response)
    }
}

lazy_static!{
    static ref EMPTY_TARBALL_BYTES: Vec<u8> = {
        let mut empty_tarball = vec![];
        {
            let mut ar = tar::Builder::new(GzEncoder::new(&mut empty_tarball, Compression::default()));
            t!(ar.finish());
        }
        empty_tarball
    };
}

/// A builder for constructing a crate for the purposes of testing publishing. If you only need
/// a crate to exist and don't need to test behavior caused by the publish request, inserting
/// a crate into the database directly by using CrateBuilder will be faster.
struct PublishBuilder {
    krate_name: String,
    version: semver::Version,
    tarball: Vec<u8>,
}

impl PublishBuilder {
    /// Create a request to publish a crate with the given name, version 1.0.0, and no files
    /// in its tarball.
    fn new(krate_name: &str) -> Self {
        PublishBuilder {
            krate_name: krate_name.into(),
            version: semver::Version::parse("1.0.0").unwrap(),
            tarball: EMPTY_TARBALL_BYTES.to_vec(),
        }
    }

    /// Set the version of the crate being published to something other than the default of 1.0.0.
    fn version(mut self, version: &str) -> Self {
        self.version = semver::Version::parse(version).unwrap();
        self
    }

    /// Set the files in the crate's tarball.
    fn files(mut self, files: &[(&str, &[u8])]) -> Self {
        let mut slices = files.iter().map(|p| p.1).collect::<Vec<_>>();
        let files = files
            .iter()
            .zip(&mut slices)
            .map(|(&(name, _), data)| {
                let len = data.len() as u64;
                (name, data as &mut Read, len)
            })
            .collect::<Vec<_>>();

        let mut tarball = Vec::new();
        {
            let mut ar = tar::Builder::new(GzEncoder::new(&mut tarball, Compression::default()));
            for (name, ref mut data, size) in files {
                let mut header = tar::Header::new_gnu();
                t!(header.set_path(name));
                header.set_size(size);
                header.set_cksum();
                t!(ar.append(&header, data));
            }
            t!(ar.finish());
        }

        self.tarball = tarball;
        self
    }

    /// Publish the crate in the context of the given session, which must have a logged in user.
    fn publish(self, session: &mut BrowserSession) {
        let new_crate = u::NewCrate {
            name: u::CrateName(self.krate_name.clone()),
            vers: u::CrateVersion(self.version),
            features: HashMap::new(),
            deps: Vec::new(),
            authors: vec!["foo".to_string()],
            description: Some("description".to_string()),
            homepage: None,
            documentation: None,
            readme: None,
            readme_file: None,
            keywords: Some(u::KeywordList(Vec::new())),
            categories: Some(u::CategoryList(Vec::new())),
            license: Some("MIT".to_string()),
            license_file: None,
            repository: None,
            badges: Some(HashMap::new()),
            links: None,
        };

        session.request
            .with_method(Method::Put).with_path("/api/v1/crates/new")
            .with_body(&::new_crate_to_body_with_tarball(&new_crate, &self.tarball));

        let mut response = session.make_request();
        let json: GoodCrate = json(&mut response);
        assert_eq!(json.krate.name, self.krate_name);
    }
}
