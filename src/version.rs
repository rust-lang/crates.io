use std::collections::HashMap;

use conduit::{Request, Response};
use conduit_router::RequestParams;
use pg::GenericConnection;
use pg::rows::Row;
use rustc_serialize::json;
use semver;
use time::Duration;
use time::Timespec;
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
use util::{RequestUtils, CargoResult, ChainError, internal, human};

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
}

impl Version {
    pub fn find_by_num(conn: &GenericConnection,
                       crate_id: i32,
                       num: &semver::Version)
                       -> CargoResult<Option<Version>> {
        let num = num.to_string();
        let stmt = conn.prepare("SELECT * FROM versions \
                                      WHERE crate_id = $1 AND num = $2")?;
        let rows = stmt.query(&[&crate_id, &num])?;
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
        let stmt = conn.prepare("INSERT INTO versions \
                                      (crate_id, num, features) \
                                      VALUES ($1, $2, $3) \
                                      RETURNING *")?;
        let rows = stmt.query(&[&crate_id, &num, &features])?;
        let ret: Version = Model::from_row(&rows.iter().next().chain_error(|| {
            internal("no version returned")
        })?);
        for author in authors.iter() {
            ret.add_author(conn, &author)?;
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
        let krate = Crate::find_by_name(conn, name).map_err(|_| {
            human(format!("no known crate named `{}`", &**name))
        })?;
        if dep.version_req.0 == semver::VersionReq::parse("*").unwrap() {
            return Err(human(format!("wildcard (`*`) dependency constraints are not allowed \
                                      on crates.io. See http://doc.crates.io/faq.html#can-\
                                      libraries-use--as-a-version-for-their-dependencies for more \
                                      information")));
        }
        let features: Vec<String> = dep.features.iter().map(|s| {
            s[..].to_string()
        }).collect();
        let dep = Dependency::insert(conn, self.id, krate.id,
                                     &*dep.version_req,
                                     dep.kind.unwrap_or(Kind::Normal),
                                     dep.optional,
                                     dep.default_features,
                                     &features,
                                     &dep.target)?;
        Ok((dep, krate))
    }

    /// Returns (dependency, crate dependency name)
    pub fn dependencies(&self, conn: &GenericConnection)
                        -> CargoResult<Vec<(Dependency, String)>> {
        let stmt = conn.prepare("SELECT dependencies.*,
                                             crates.name AS crate_name
                                      FROM dependencies
                                      LEFT JOIN crates
                                        ON crates.id = dependencies.crate_id
                                      WHERE dependencies.version_id = $1")?;
        let rows = stmt.query(&[&self.id])?;
        Ok(rows.iter().map(|r| {
            (Model::from_row(&r), r.get("crate_name"))
        }).collect())
    }

    pub fn authors(&self, conn: &GenericConnection) -> CargoResult<Vec<Author>> {
        let stmt = conn.prepare("SELECT * FROM version_authors
                                       WHERE version_id = $1")?;
        let rows = stmt.query(&[&self.id])?;
        let mut authors = rows.into_iter().map(|row| {
            let user_id: Option<i32> = row.get("user_id");
            let name: String = row.get("name");
            Ok(match user_id {
                Some(id) => Author::User(User::find(conn, id)?),
                None => Author::Name(name),
            })
        }).collect::<CargoResult<Vec<Author>>>()?;
        authors.sort_by(|ref a, ref b| a.name().cmp(&b.name()));
        Ok(authors)
    }

    pub fn add_author(&self,
                      conn: &GenericConnection,
                      name: &str) -> CargoResult<()> {
        println!("add author: {}", name);
        // TODO: at least try to link `name` to a pre-existing user
        conn.execute("INSERT INTO version_authors (version_id, name)
                           VALUES ($1, $2)", &[&self.id, &name])?;
        Ok(())
    }

    pub fn yank(&self, conn: &GenericConnection, yanked: bool) -> CargoResult<()> {
        conn.execute("UPDATE versions SET yanked = $1 WHERE id = $2",
                     &[&yanked, &self.id])?;
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

impl Author {
    fn name(&self) -> Option<&str> {
        match self {
            &Author::Name(ref n) => {Some(&n)},
            &Author::User(ref u) => {u.name.as_ref().map(String::as_str)}
        }
    }
}

/// Handles the `GET /versions` route.
pub fn index(req: &mut Request) -> CargoResult<Response> {
    let conn = req.tx()?;

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
        let stmt = conn.prepare("\
            SELECT versions.*, crates.name AS crate_name
              FROM versions
            LEFT JOIN crates ON crates.id = versions.crate_id
            WHERE versions.id = ANY($1)
        ")?;
        for row in stmt.query(&[&ids])?.iter() {
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
        Some(..) => version_and_crate(req)?,
        None => {
            let id = &req.params()["version_id"];
            let id = id.parse().unwrap_or(0);
            let conn = req.tx()?;
            let version = Version::find(&*conn, id)?;
            let krate = Crate::find(&*conn, version.crate_id)?;
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
    let semver = semver::Version::parse(semver).map_err(|_| {
        human(format!("invalid semver: {}", semver))
    })?;
    let tx = req.tx()?;
    let krate = Crate::find_by_name(tx, crate_name)?;
    let version = Version::find_by_num(tx, krate.id, &semver)?;
    let version = version.chain_error(|| {
        human(format!("crate `{}` does not have a version `{}`",
                      crate_name, semver))
    })?;
    Ok((version, krate))
}

/// Handles the `GET /crates/:crate_id/:version/dependencies` route.
pub fn dependencies(req: &mut Request) -> CargoResult<Response> {
    let (version, _) = version_and_crate(req)?;
    let tx = req.tx()?;
    let deps = version.dependencies(tx)?;
    let deps = deps.into_iter().map(|(dep, crate_name)| {
        dep.encodable(&crate_name, None)
    }).collect();

    #[derive(RustcEncodable)]
    struct R { dependencies: Vec<EncodableDependency> }
    Ok(req.json(&R{ dependencies: deps }))
}

/// Handles the `GET /crates/:crate_id/:version/downloads` route.
pub fn downloads(req: &mut Request) -> CargoResult<Response> {
    let (version, _) = version_and_crate(req)?;

    let tx = req.tx()?;
    let cutoff_date = ::now() + Duration::days(-90);
    let stmt = tx.prepare("SELECT * FROM version_downloads
                                WHERE date > $1 AND version_id = $2
                                ORDER BY date ASC")?;
    let mut downloads = Vec::new();
    for row in stmt.query(&[&cutoff_date, &version.id])?.iter() {
        let download: VersionDownload = Model::from_row(&row);
        downloads.push(download.encodable());
    }

    #[derive(RustcEncodable)]
    struct R { version_downloads: Vec<EncodableVersionDownload> }
    Ok(req.json(&R{ version_downloads: downloads }))
}

/// Handles the `GET /crates/:crate_id/:version/authors` route.
pub fn authors(req: &mut Request) -> CargoResult<Response> {
    let (version, _) = version_and_crate(req)?;
    let tx = req.tx()?;
    let (mut users, mut names) = (Vec::new(), Vec::new());
    for author in version.authors(tx)?.into_iter() {
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
    let (version, krate) = version_and_crate(req)?;
    let user = req.user()?;
    let tx = req.tx()?;
    let owners = krate.owners(tx)?;
    if rights(req.app(), &owners, &user)? < Rights::Publish {
        return Err(human("must already be an owner to yank or unyank"))
    }

    if version.yanked != yanked {
        version.yank(tx, yanked)?;
        git::yank(&**req.app(), &krate.name, &version.num, yanked)?;
    }

    #[derive(RustcEncodable)]
    struct R { ok: bool }
    Ok(req.json(&R{ ok: true }))
}
