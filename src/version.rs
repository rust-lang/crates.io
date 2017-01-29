use std::collections::{HashMap, BTreeSet};
use std::str::FromStr;

use conduit::{Request, Response};
use conduit_router::RequestParams;
use pg::GenericConnection;
use pg::rows::Row;
use rustc_serialize::json;
use semver;
use time::{self, Duration, Timespec};
use url;

use {Model, Crate, User};
use app::RequestApp;
use db::RequestTransaction;
use dependency::{Dependency, EncodableDependency, Kind};
use download::{VersionDownload, EncodableVersionDownload};
use git;
use upload;
use user::RequestUser;
use owner::{rights, Rights};
use util::{RequestUtils, CargoResult, ChainError, internal, human, CargoError};

#[derive(Clone)]
pub struct Version {
    pub id: i32,
    pub crate_id: i32,
    pub num: semver::Version,
    pub updated_at: Timespec,
    pub created_at: Timespec,
    pub downloads: i32,
    pub features: HashMap<String, Vec<String>>,
    pub yanked: bool,
}

pub enum Author {
    User(User),
    Name(String),
}

#[derive(RustcEncodable, RustcDecodable)]
pub struct EncodableVersion {
    pub id: i32,
    pub krate: String,
    pub num: String,
    pub dl_path: String,
    pub updated_at: String,
    pub created_at: String,
    pub downloads: i32,
    pub features: HashMap<String, Vec<String>>,
    pub yanked: bool,
    pub links: VersionLinks,
}

#[derive(RustcEncodable, RustcDecodable)]
pub struct VersionLinks {
    pub dependencies: String,
    pub version_downloads: String,
    pub authors: String,
    pub build_info: String,
}

#[derive(RustcEncodable, RustcDecodable, Default)]
pub struct EncodableBuildInfo {
    id: i32,
    pub ordering: HashMap<String, Vec<String>>,
    pub stable: HashMap<String, HashMap<String, bool>>,
    pub beta: HashMap<String, HashMap<String, bool>>,
    pub nightly: HashMap<String, HashMap<String, bool>>,
}

pub enum ChannelVersion {
    Stable(semver::Version),
    Beta(Timespec),
    Nightly(Timespec),
}

impl FromStr for ChannelVersion {
    type Err = Box<CargoError>;

    fn from_str(s: &str) -> CargoResult<Self> {
        // Recognized formats:
        // rustc 1.14.0 (e8a012324 2016-12-16)
        // rustc 1.15.0-beta.5 (10893a9a3 2017-01-19)
        // rustc 1.16.0-nightly (df8debf6d 2017-01-25)

        let pieces: Vec<_> = s.split(&[' ', '(', ')'][..])
                              .filter(|s| !s.trim().is_empty())
                              .collect();
        if pieces.len() != 4 {
            return Err(human(format!(
                "rust_version `{}` not recognized; \
                expected format like `rustc X.Y.Z (SHA YYYY-MM-DD)`",
                s
            )));
        }

        if pieces[1].contains("nightly") {
            Ok(ChannelVersion::Nightly(time::strptime(pieces[3], "%Y-%m-%d")?.to_timespec()))
        } else if pieces[1].contains("beta") {
            Ok(ChannelVersion::Beta(time::strptime(pieces[3], "%Y-%m-%d")?.to_timespec()))
        } else {
            let v = semver::Version::parse(pieces[1])?;
            if v.pre.is_empty() {
                Ok(ChannelVersion::Stable(v))
            } else {
                return Err(human(format!(
                    "rust_version `{}` not recognized as nightly, beta, or stable",
                    s
                )));
            }
        }
    }
}

impl Version {
    pub fn find_by_num(conn: &GenericConnection,
                       crate_id: i32,
                       num: &semver::Version)
                       -> CargoResult<Option<Version>> {
        let num = num.to_string();
        let stmt = try!(conn.prepare("SELECT * FROM versions \
                                      WHERE crate_id = $1 AND num = $2"));
        let rows = try!(stmt.query(&[&crate_id, &num]));
        Ok(rows.iter().next().map(|r| Model::from_row(&r)))
    }

    pub fn insert(conn: &GenericConnection,
                  crate_id: i32,
                  num: &semver::Version,
                  features: &HashMap<String, Vec<String>>,
                  authors: &[String])
                  -> CargoResult<Version> {
        let num = num.to_string();
        let features = json::encode(features).unwrap();
        let stmt = try!(conn.prepare("INSERT INTO versions \
                                      (crate_id, num, features) \
                                      VALUES ($1, $2, $3) \
                                      RETURNING *"));
        let rows = try!(stmt.query(&[&crate_id, &num, &features]));
        let ret: Version = Model::from_row(&try!(rows.iter().next().chain_error(|| {
            internal("no version returned")
        })));
        for author in authors.iter() {
            try!(ret.add_author(conn, &author));
        }
        Ok(ret)
    }

    pub fn valid(version: &str) -> bool {
        semver::Version::parse(version).is_ok()
    }

    pub fn encodable(self, crate_name: &str) -> EncodableVersion {
        let Version { id, crate_id: _, num, updated_at, created_at,
                      downloads, features, yanked } = self;
        let num = num.to_string();
        EncodableVersion {
            dl_path: format!("/api/v1/crates/{}/{}/download", crate_name, num),
            num: num.clone(),
            id: id,
            krate: crate_name.to_string(),
            updated_at: ::encode_time(updated_at),
            created_at: ::encode_time(created_at),
            downloads: downloads,
            features: features,
            yanked: yanked,
            links: VersionLinks {
                dependencies: format!("/api/v1/crates/{}/{}/dependencies",
                                      crate_name, num),
                version_downloads: format!("/api/v1/crates/{}/{}/downloads",
                                           crate_name, num),
                authors: format!("/api/v1/crates/{}/{}/authors", crate_name, num),
                build_info: format!("/api/v1/crates/{}/{}/build_info", crate_name, num),
            },
        }
    }

    /// Add a dependency to this version, returning both the dependency and the
    /// crate that the dependency points to
    pub fn add_dependency(&mut self,
                          conn: &GenericConnection,
                          dep: &upload::CrateDependency)
                          -> CargoResult<(Dependency, Crate)> {
        let name = &dep.name;
        let krate = try!(Crate::find_by_name(conn, name).map_err(|_| {
            human(format!("no known crate named `{}`", &**name))
        }));
        if dep.version_req.0 == semver::VersionReq::parse("*").unwrap() {
            return Err(human(format!("wildcard (`*`) dependency constraints are not allowed \
                                      on crates.io. See http://doc.crates.io/faq.html#can-\
                                      libraries-use--as-a-version-for-their-dependencies for more \
                                      information")));
        }
        let features: Vec<String> = dep.features.iter().map(|s| {
            s[..].to_string()
        }).collect();
        let dep = try!(Dependency::insert(conn, self.id, krate.id,
                                          &*dep.version_req,
                                          dep.kind.unwrap_or(Kind::Normal),
                                          dep.optional,
                                          dep.default_features,
                                          &features,
                                          &dep.target));
        Ok((dep, krate))
    }

    /// Returns (dependency, crate dependency name)
    pub fn dependencies(&self, conn: &GenericConnection)
                        -> CargoResult<Vec<(Dependency, String)>> {
        let stmt = try!(conn.prepare("SELECT dependencies.*,
                                             crates.name AS crate_name
                                      FROM dependencies
                                      LEFT JOIN crates
                                        ON crates.id = dependencies.crate_id
                                      WHERE dependencies.version_id = $1"));
        let rows = try!(stmt.query(&[&self.id]));
        Ok(rows.iter().map(|r| {
            (Model::from_row(&r), r.get("crate_name"))
        }).collect())
    }

    pub fn authors(&self, conn: &GenericConnection) -> CargoResult<Vec<Author>> {
        let stmt = try!(conn.prepare("SELECT * FROM version_authors
                                       WHERE version_id = $1"));
        let rows = try!(stmt.query(&[&self.id]));
        rows.into_iter().map(|row| {
            let user_id: Option<i32> = row.get("user_id");
            let name: String = row.get("name");
            Ok(match user_id {
                Some(id) => Author::User(try!(User::find(conn, id))),
                None => Author::Name(name),
            })
        }).collect()
    }

    pub fn add_author(&self,
                      conn: &GenericConnection,
                      name: &str) -> CargoResult<()> {
        println!("add author: {}", name);
        // TODO: at least try to link `name` to a pre-existing user
        try!(conn.execute("INSERT INTO version_authors (version_id, name)
                           VALUES ($1, $2)", &[&self.id, &name]));
        Ok(())
    }

    pub fn yank(&self, conn: &GenericConnection, yanked: bool) -> CargoResult<()> {
        try!(conn.execute("UPDATE versions SET yanked = $1 WHERE id = $2",
                          &[&yanked, &self.id]));
        Ok(())
    }

    pub fn store_build_info(&self,
                            conn: &GenericConnection,
                            info: upload::VersionBuildInfo,
                            krate: &Crate) -> CargoResult<()> {

        // Verify specified Rust version will parse before doing any inserting
        let channel_version = info.channel_version()?;

        // Future improvement: allow overwriting an existing row, in which case
        // the ON CONFLICT DO should set `passed` instead of doing nothing.
        let inserted = conn.execute("INSERT INTO build_info \
                          (version_id, rust_version, target, passed) \
                      VALUES ($1, $2, $3, $4) \
                      ON CONFLICT (version_id, rust_version, target) DO NOTHING",
            &[&self.id, &info.rust_version, &info.target, &info.passed])?;

        if inserted == 0 {
            return Err(human(format!(
                "Build info already specified for {} v{} with {} and target {}",
                krate.name, self.num, info.rust_version, info.target)));
        }

        if info.passed && self.num == krate.max_version {
            match channel_version {
                ChannelVersion::Nightly(date) => {
                    let should_update = krate.max_build_info_nightly.as_ref().map(|max| {
                        date > *max
                    }).unwrap_or(true);

                    if should_update {
                        conn.execute("UPDATE crates \
                                      SET max_build_info_nightly = $1 \
                                      WHERE id = $2",
                                      &[&date, &krate.id])?;
                    }
                },
                ChannelVersion::Beta(date) => {
                    let should_update = krate.max_build_info_beta.as_ref().map(|max| {
                        date > *max
                    }).unwrap_or(true);

                    if should_update {
                        conn.execute("UPDATE crates \
                                      SET max_build_info_beta = $1 \
                                      WHERE id = $2",
                                      &[&date, &krate.id])?;
                    }
                },
                ChannelVersion::Stable(vers) => {
                    let should_update = krate.max_build_info_stable.as_ref().map(|max| {
                        vers > *max
                    }).unwrap_or(true);

                    if should_update {
                        conn.execute("UPDATE crates \
                                      SET max_build_info_stable = $1 \
                                      WHERE id = $2",
                                      &[&vers.to_string(), &krate.id])?;
                    }
                },
            }
        }

        Ok(())
    }
}

impl Model for Version {
    fn from_row(row: &Row) -> Version {
        let num: String = row.get("num");
        let features: Option<String> = row.get("features");
        let features = features.map(|s| {
            json::decode(&s).unwrap()
        }).unwrap_or_else(|| HashMap::new());
        Version {
            id: row.get("id"),
            crate_id: row.get("crate_id"),
            num: semver::Version::parse(&num).unwrap(),
            updated_at: row.get("updated_at"),
            created_at: row.get("created_at"),
            downloads: row.get("downloads"),
            features: features,
            yanked: row.get("yanked"),
        }
    }
    fn table_name(_: Option<Version>) -> &'static str { "versions" }
}

/// Handles the `GET /versions` route.
pub fn index(req: &mut Request) -> CargoResult<Response> {
    let conn = try!(req.tx());

    // Extract all ids requested.
    let query = url::form_urlencoded::parse(req.query_string().unwrap_or("")
                                               .as_bytes());
    let ids = query.filter_map(|(ref a, ref b)| {
        if *a == "ids[]" {
            b.parse().ok()
        } else {
            None
        }
    }).collect::<Vec<i32>>();

    // Load all versions
    //
    // TODO: can rust-postgres do this for us?
    let mut versions = Vec::new();
    if ids.len() > 0 {
        let stmt = try!(conn.prepare("\
            SELECT versions.*, crates.name AS crate_name
              FROM versions
            LEFT JOIN crates ON crates.id = versions.crate_id
            WHERE versions.id = ANY($1)
        "));
        for row in try!(stmt.query(&[&ids])).iter() {
            let v: Version = Model::from_row(&row);
            let crate_name: String = row.get("crate_name");
            versions.push(v.encodable(&crate_name));
        }
    }

    #[derive(RustcEncodable)]
    struct R { versions: Vec<EncodableVersion> }
    Ok(req.json(&R { versions: versions }))
}

/// Handles the `GET /versions/:version_id` route.
pub fn show(req: &mut Request) -> CargoResult<Response> {
    let (version, krate) = match req.params().find("crate_id") {
        Some(..) => try!(version_and_crate(req)),
        None => {
            let id = &req.params()["version_id"];
            let id = id.parse().unwrap_or(0);
            let conn = try!(req.tx());
            let version = try!(Version::find(&*conn, id));
            let krate = try!(Crate::find(&*conn, version.crate_id));
            (version, krate)
        }
    };

    #[derive(RustcEncodable)]
    struct R { version: EncodableVersion }
    Ok(req.json(&R { version: version.encodable(&krate.name) }))
}

fn version_and_crate(req: &mut Request) -> CargoResult<(Version, Crate)> {
    let crate_name = &req.params()["crate_id"];
    let semver = &req.params()["version"];
    let semver = try!(semver::Version::parse(semver).map_err(|_| {
        human(format!("invalid semver: {}", semver))
    }));
    let tx = try!(req.tx());
    let krate = try!(Crate::find_by_name(tx, crate_name));
    let version = try!(Version::find_by_num(tx, krate.id, &semver));
    let version = try!(version.chain_error(|| {
        human(format!("crate `{}` does not have a version `{}`",
                      crate_name, semver))
    }));
    Ok((version, krate))
}

/// Handles the `GET /crates/:crate_id/:version/dependencies` route.
pub fn dependencies(req: &mut Request) -> CargoResult<Response> {
    let (version, _) = try!(version_and_crate(req));
    let tx = try!(req.tx());
    let deps = try!(version.dependencies(tx));
    let deps = deps.into_iter().map(|(dep, crate_name)| {
        dep.encodable(&crate_name)
    }).collect();

    #[derive(RustcEncodable)]
    struct R { dependencies: Vec<EncodableDependency> }
    Ok(req.json(&R{ dependencies: deps }))
}

/// Handles the `GET /crates/:crate_id/:version/downloads` route.
pub fn downloads(req: &mut Request) -> CargoResult<Response> {
    let (version, _) = try!(version_and_crate(req));

    let tx = try!(req.tx());
    let cutoff_date = ::now() + Duration::days(-90);
    let stmt = try!(tx.prepare("SELECT * FROM version_downloads
                                WHERE date > $1 AND version_id = $2
                                ORDER BY date ASC"));
    let mut downloads = Vec::new();
    for row in try!(stmt.query(&[&cutoff_date, &version.id])).iter() {
        let download: VersionDownload = Model::from_row(&row);
        downloads.push(download.encodable());
    }

    #[derive(RustcEncodable)]
    struct R { version_downloads: Vec<EncodableVersionDownload> }
    Ok(req.json(&R{ version_downloads: downloads }))
}

/// Handles the `GET /crates/:crate_id/:version/build_info` route.
pub fn build_info(req: &mut Request) -> CargoResult<Response> {
    let (version, _) = try!(version_and_crate(req));

    let tx = try!(req.tx());

    let stmt = try!(tx.prepare("SELECT * FROM build_info
                                WHERE version_id = $1"));
    let rows = stmt.query(&[&version.id])?;

    let mut build_info = EncodableBuildInfo::default();
    build_info.id = version.id;
    let mut stables = BTreeSet::new();
    let mut betas = BTreeSet::new();
    let mut nightlies = BTreeSet::new();

    for row in &rows {
        let rust_version: String = row.get("rust_version");
        let rust_version: ChannelVersion = rust_version.parse()?;
        let target = row.get("target");
        let passed = row.get("passed");

        match rust_version {
            ChannelVersion::Stable(semver) => {
                let key = semver.to_string();
                stables.insert(semver);
                build_info.stable.entry(key)
                                 .or_insert_with(HashMap::new)
                                 .insert(target, passed);
            }
            ChannelVersion::Beta(date) => {
                let key = ::encode_time(date);
                betas.insert(date);
                build_info.beta.entry(key)
                                  .or_insert_with(HashMap::new)
                                  .insert(target, passed);
            }
            ChannelVersion::Nightly(date) => {
                let key = ::encode_time(date);
                nightlies.insert(date);
                build_info.nightly.entry(key)
                                  .or_insert_with(HashMap::new)
                                  .insert(target, passed);
            }
        }
    }

    build_info.ordering.insert(
        String::from("stable"),
        stables.into_iter().map(|s| s.to_string()).collect()
    );

    build_info.ordering.insert(
        String::from("beta"),
        betas.into_iter().map(::encode_time).collect()
    );

    build_info.ordering.insert(
        String::from("nightly"),
        nightlies.into_iter().map(::encode_time).collect()
    );

    #[derive(RustcEncodable)]
    struct R { build_info: EncodableBuildInfo }
    Ok(req.json(&R{ build_info: build_info }))
}

/// Handles the `GET /crates/:crate_id/:version/authors` route.
pub fn authors(req: &mut Request) -> CargoResult<Response> {
    let (version, _) = try!(version_and_crate(req));
    let tx = try!(req.tx());
    let (mut users, mut names) = (Vec::new(), Vec::new());
    for author in try!(version.authors(tx)).into_iter() {
        match author {
            Author::User(u) => users.push(u.encodable()),
            Author::Name(n) => names.push(n),
        }
    }

    #[derive(RustcEncodable)]
    struct R { users: Vec<::user::EncodableUser>, meta: Meta }
    #[derive(RustcEncodable)]
    struct Meta { names: Vec<String> }
    Ok(req.json(&R{ users: users, meta: Meta { names: names } }))
}

/// Handles the `DELETE /crates/:crate_id/:version/yank` route.
pub fn yank(req: &mut Request) -> CargoResult<Response> {
    modify_yank(req, true)
}

/// Handles the `PUT /crates/:crate_id/:version/unyank` route.
pub fn unyank(req: &mut Request) -> CargoResult<Response> {
    modify_yank(req, false)
}

fn modify_yank(req: &mut Request, yanked: bool) -> CargoResult<Response> {
    let (version, krate) = try!(version_and_crate(req));
    let user = try!(req.user());
    let tx = try!(req.tx());
    let owners = try!(krate.owners(tx));
    if try!(rights(req.app(), &owners, &user)) < Rights::Publish {
        return Err(human("must already be an owner to yank or unyank"))
    }

    if version.yanked != yanked {
        try!(version.yank(tx, yanked));
        try!(git::yank(&**req.app(), &krate.name, &version.num, yanked));
    }

    #[derive(RustcEncodable)]
    struct R { ok: bool }
    Ok(req.json(&R{ ok: true }))
}

/// Handles the `POST /crates/:crate_id/:version/build_info` route.
pub fn publish_build_info(req: &mut Request) -> CargoResult<Response> {
    let mut body = String::new();
    try!(req.body().read_to_string(&mut body));
    let info: upload::VersionBuildInfo = try!(json::decode(&body).map_err(|e| {
        human(format!("invalid upload request: {:?}", e))
    }));

    let (version, krate) = try!(version_and_crate(req));
    let user = try!(req.user());
    let tx = try!(req.tx());
    let owners = try!(krate.owners(tx));
    if try!(rights(req.app(), &owners, &user)) < Rights::Publish {
        return Err(human("must already be an owner to publish build info"))
    }

    version.store_build_info(tx, info, &krate)?;

    #[derive(RustcEncodable)]
    struct R { ok: bool }
    Ok(req.json(&R{ ok: true }))
}
