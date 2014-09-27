use std::collections::HashMap;
use std::sync::Arc;
use serialize::json;
use serialize::hex::ToHex;
use time::Timespec;

use conduit::{Request, Response};
use conduit_router::RequestParams;
use curl::http;
use pg::types::ToSql;
use pg::{PostgresRow, PostgresStatement};
use semver;

use app::{App, RequestApp};
use db::{Connection, RequestTransaction};
use dependency::Dependency;
use git;
use user::User;
use util::{RequestUtils, CargoResult, Require, internal, ChainError, human};
use util::{LimitErrorReader, HashingReader};
use util::errors::{NotFound, CargoError};
use version::{Version, EncodableVersion};

#[deriving(Clone)]
pub struct Crate {
    pub id: i32,
    pub name: String,
    pub user_id: i32,
    pub updated_at: Timespec,
    pub created_at: Timespec,
    pub downloads: i32,
    pub max_version: semver::Version,
}

#[deriving(Encodable, Decodable)]
pub struct EncodableCrate {
    pub id: String,
    pub name: String,
    pub versions: Vec<i32>,
    pub updated_at: String,
    pub created_at: String,
    pub downloads: i32,
    pub max_version: String,
}

impl Crate {
    pub fn from_row(row: &PostgresRow) -> Crate {
        let max: String = row.get("max_version");
        Crate {
            id: row.get("id"),
            name: row.get("name"),
            user_id: row.get("user_id"),
            updated_at: row.get("updated_at"),
            created_at: row.get("created_at"),
            downloads: row.get("downloads"),
            max_version: semver::Version::parse(max.as_slice()).unwrap(),
        }
    }

    pub fn find(conn: &Connection, id: i32) -> CargoResult<Crate> {
        let stmt = try!(conn.prepare("SELECT * FROM crates \
                                      WHERE id = $1"));
        match try!(stmt.query(&[&id])).next() {
            Some(row) => Ok(Crate::from_row(&row)),
            None => Err(NotFound.box_error()),
        }
    }

    pub fn find_by_name(conn: &Connection, name: &str) -> CargoResult<Crate> {
        let stmt = try!(conn.prepare("SELECT * FROM crates \
                                      WHERE name = $1 LIMIT 1"));
        match try!(stmt.query(&[&name as &ToSql])).next() {
            Some(row) => Ok(Crate::from_row(&row)),
            None => Err(NotFound.box_error()),
        }
    }

    pub fn find_or_insert(conn: &Connection, name: &str,
                          user_id: i32) -> CargoResult<Crate> {
        // TODO: like with users, this is sadly racy

        let stmt = try!(conn.prepare("SELECT * FROM crates WHERE name = $1"));
        let mut rows = try!(stmt.query(&[&name as &ToSql]));
        match rows.next() {
            Some(row) => return Ok(Crate::from_row(&row)),
            None => {}
        }
        let stmt = try!(conn.prepare("INSERT INTO crates \
                                      (name, user_id, created_at,
                                       updated_at, downloads, max_version) \
                                      VALUES ($1, $2, $3, $4, 0, '0.0.0') \
                                      RETURNING *"));
        let now = ::now();
        let mut rows = try!(stmt.query(&[&name as &ToSql, &user_id, &now, &now]));
        Ok(Crate::from_row(&try!(rows.next().require(|| {
            internal("no crate returned")
        }))))
    }

    pub fn valid_name(name: &str) -> bool {
        if name.len() == 0 { return false }
        name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }

    fn encodable(self, versions: Vec<i32>) -> EncodableCrate {
        let Crate { name, created_at, updated_at, downloads,
                      max_version, .. } = self;
        EncodableCrate {
            id: name.clone(),
            name: name,
            versions: versions,
            updated_at: ::encode_time(updated_at),
            created_at: ::encode_time(created_at),
            downloads: downloads,
            max_version: max_version.to_string(),
        }
    }

    pub fn versions(&self, conn: &Connection) -> CargoResult<Vec<Version>> {
        let stmt = try!(conn.prepare("SELECT * FROM versions \
                                      WHERE crate_id = $1"));
        let rows = try!(stmt.query(&[&self.id]));
        Ok(rows.map(|r| Version::from_row(&r)).collect())
    }

    pub fn dl_path(&self, version: &str) -> String {
        format!("/crates/{}/{}/download", self.name, version)
    }

    pub fn s3_path(&self, version: &str) -> String {
        format!("/pkg/{}/{}-{}.tar.gz", self.name, self.name, version)
    }

    pub fn encode_many(conn: &Connection, crates: Vec<Crate>)
                       -> CargoResult<Vec<EncodableCrate>> {
        // TODO: can rust-postgres do this escaping?
        let crateids: Vec<i32> = crates.iter().map(|p| p.id).collect();
        let mut map = HashMap::new();
        let query = format!("'{{{:#}}}'::int[]", crateids.as_slice());
        let stmt = try!(conn.prepare(format!("SELECT id, crate_id FROM versions \
                                              WHERE crate_id = ANY({})",
                                             query).as_slice()));
        for row in try!(stmt.query(&[])) {
            map.find_or_insert(row.get("crate_id"), Vec::new())
               .push(row.get("id"));
        }

        Ok(crates.into_iter().map(|p| {
            let id = p.id;
            p.encodable(map.pop(&id).unwrap())
        }).collect())
    }

    pub fn add_version(&self, conn: &Connection, num: &str)
                       -> CargoResult<Version> {
        let ver = semver::Version::parse(num).unwrap();
        let max = if ver > self.max_version {&ver} else {&self.max_version};
        let max = max.to_string();
        try!(conn.execute("UPDATE crates SET updated_at = $1, max_version = $2
                           WHERE id = $3",
                          &[&::now(), &max, &self.id]));
        Version::insert(conn, self.id, num)
    }
}

pub fn index(req: &mut Request) -> CargoResult<Response> {
    let conn = try!(req.tx());
    let query = req.query();
    let page = query.find_equiv(&"page").map(|s| s.as_slice())
                    .and_then(from_str::<i64>).unwrap_or(1);
    let limit = query.find_equiv(&"per_page").map(|s| s.as_slice())
                     .and_then(from_str::<i64>).unwrap_or(10);
    if limit > 100 { return Err(human("cannot request more than 100 crates")) }
    let offset = (page - 1) * limit;
    let pattern = query.find_equiv(&"letter")
                       .map(|s| s.as_slice().char_at(0).to_lowercase())
                       .map(|s| format!("{}%", s))
                       .unwrap_or("%".to_string());

    // Collect all the crates
    let stmt = try!(conn.prepare("SELECT * FROM crates \
                                  WHERE name LIKE $3 \
                                  LIMIT $1 OFFSET $2"));
    let mut crates = Vec::new();
    for row in try!(stmt.query(&[&limit, &offset, &pattern])) {
        crates.push(Crate::from_row(&row));
    }
    let crates = try!(Crate::encode_many(conn, crates));

    // Query for the total count of crates
    let stmt = try!(conn.prepare("SELECT COUNT(*) FROM crates \
                                  WHERE name LIKE $1"));
    let row = try!(stmt.query(&[&pattern])).next().unwrap();
    let total = row.get(0u);

    #[deriving(Encodable)]
    struct R { crates: Vec<EncodableCrate>, meta: Meta }
    #[deriving(Encodable)]
    struct Meta { total: i64 }

    Ok(req.json(&R {
        crates: crates,
        meta: Meta { total: total },
    }))
}

pub fn summary(req: &mut Request) -> CargoResult<Response> {
    let tx = try!(req.tx());
    let num_crates = {
        let stmt = try!(tx.prepare("SELECT COUNT(*) FROM crates"));
        let mut rows = try!(stmt.query(&[]));
        rows.next().unwrap().get("count")
    };
    let num_downloads = {
        let stmt = try!(tx.prepare("SELECT total_downloads FROM metadata"));
        let mut rows = try!(stmt.query(&[]));
        rows.next().unwrap().get("total_downloads")
    };

    let to_crates = |stmt: PostgresStatement| {
        let rows = try!(stmt.query([]));
        Crate::encode_many(tx, rows.map(|r| Crate::from_row(&r)).collect())
    };
    let new_crates = try!(tx.prepare("SELECT * FROM crates \
                                        ORDER BY created_at DESC LIMIT 10"));
    let just_updated = try!(tx.prepare("SELECT * FROM crates \
                                        ORDER BY updated_at DESC LIMIT 10"));
    let most_downloaded = try!(tx.prepare("SELECT * FROM crates \
                                           ORDER BY downloads DESC LIMIT 10"));

    #[deriving(Encodable)]
    struct R {
        num_downloads: i64,
        num_crates: i64,
        new_crates: Vec<EncodableCrate>,
        most_downloaded: Vec<EncodableCrate>,
        just_updated: Vec<EncodableCrate>,
    }
    Ok(req.json(&R {
        num_downloads: num_downloads,
        num_crates: num_crates,
        new_crates: try!(to_crates(new_crates)),
        most_downloaded: try!(to_crates(most_downloaded)),
        just_updated: try!(to_crates(just_updated)),
    }))
}

pub fn show(req: &mut Request) -> CargoResult<Response> {
    let name = &req.params()["crate_id"];
    let conn = try!(req.tx());
    let krate = try!(Crate::find_by_name(&*conn, name.as_slice()));
    let versions = try!(krate.versions(&*conn));

    #[deriving(Encodable)]
    struct R { krate: EncodableCrate, versions: Vec<EncodableVersion>, }
    Ok(req.json(&R {
        krate: krate.clone().encodable(versions.iter().map(|v| v.id).collect()),
        versions: versions.into_iter().map(|v| v.encodable(&krate)).collect(),
    }))
}
#[deriving(Encodable)]
pub struct NewCrate {
    pub name: String,
    pub vers: String,
    pub deps: Vec<Dependency>,
    pub cksum: String,
    pub features: HashMap<String, Vec<String>>,
}

pub fn new(req: &mut Request) -> CargoResult<Response> {
    let app = req.app().clone();

    let (mut new_crate, user) = try!(parse_new_headers(req));

    // Persist the new crate, if it doesn't already exist
    let krate = try!(Crate::find_or_insert(try!(req.tx()),
                                           new_crate.name.as_slice(),
                                           user.id));
    if krate.user_id != user.id {
        return Err(human("crate name has already been claimed by another user"))
    }

    // Persist the new version of this crate
    match try!(Version::find_by_num(try!(req.tx()), krate.id,
                                    new_crate.vers.as_slice())) {
        Some(..) => {
            return Err(human(format!("crate version `{}` is already uploaded",
                                     new_crate.vers)))
        }
        None => {}
    }
    let vers = try!(krate.add_version(try!(req.tx()), new_crate.vers.as_slice()));

    // Link this new version to all dependencies
    for dep in new_crate.deps.iter() {
        let tx = try!(req.tx());
        let krate = try!(Crate::find_by_name(tx, dep.name.as_slice()).map_err(|_| {
            human(format!("no known crate named `{}`", dep.name))
        }));
        try!(tx.execute("INSERT INTO version_dependencies \
                         (version_id, depends_on_id) VALUES ($1, $2)",
                        &[&vers.id, &krate.id]));
    }

    // Upload the crate to S3
    let handle = http::handle();
    let mut handle = match req.app().s3_proxy {
        Some(ref proxy) => handle.proxy(proxy.as_slice()),
        None => handle,
    };
    let path = krate.s3_path(new_crate.vers.as_slice());
    let (resp, cksum) = {
        let length = req.content_length().unwrap();
        let body = LimitErrorReader::new(req.body(), app.config.max_upload_size);
        let mut body = HashingReader::new(body);
        let resp = {
            let s3req = app.bucket.put(&mut handle, path.as_slice(), &mut body,
                                       "application/x-tar")
                                  .content_length(length)
                                  .header("Content-Encoding", "gzip");
            try!(s3req.exec().chain_error(|| {
                internal(format!("failed to upload to S3: `{}`", path))
            }))
        };
        (resp, body.final())
    };
    new_crate.cksum = cksum.as_slice().to_hex();
    if resp.get_code() != 200 {
        return Err(internal(format!("failed to get a 200 response from S3: {}",
                                    resp)))
    }

    // If the git commands fail below, we shouldn't keep the crate on the
    // server.
    struct Bomb { app: Arc<App>, path: Option<String>, handle: http::Handle }
    impl Drop for Bomb {
        fn drop(&mut self) {
            match self.path {
                Some(ref path) => {
                    let _ = self.app.bucket.delete(&mut self.handle,
                                                   path.as_slice())
                                .exec();
                }
                None => {}
            }
        }
    }
    let mut bomb = Bomb { app: app.clone(), path: Some(path), handle: handle };

    // Register this crate in our local git repo.
    let krate = try!(Crate::find_by_name(try!(req.tx()),
                                         new_crate.name.as_slice()));
    try!(git::add_crate(&**req.app(), &new_crate).chain_error(|| {
        internal(format!("could not add crate `{}` to the git repo", krate.name))
    }));

    // Now that we've come this far, we're committed!
    bomb.path = None;

    #[deriving(Encodable)]
    struct R { ok: bool, krate: EncodableCrate }
    Ok(req.json(&R { ok: true, krate: krate.encodable(Vec::new()) }))
}

fn parse_new_headers(req: &mut Request) -> CargoResult<(NewCrate, User)> {
    // Peel out all input parameters
    fn header<'a>(req: &'a Request, hdr: &str) -> CargoResult<Vec<&'a str>> {
        req.headers().find(hdr).require(|| {
            human(format!("missing header: {}", hdr))
        })
    }
    let auth = try!(header(req, "X-Cargo-Auth"))[0].to_string();
    let name = try!(header(req, "X-Cargo-Crate-Name"))[0].to_string();
    let vers = try!(header(req, "X-Cargo-Crate-Version"))[0].to_string();
    let feat = try!(header(req, "X-Cargo-Crate-Feature"))[0].to_string();
    let deps = try!(req.headers().find("X-Cargo-Crate-Dep").unwrap_or(Vec::new())
                       .iter().flat_map(|s| s.as_slice().split(';'))
                       .map(Dependency::parse)
                       .collect::<CargoResult<Vec<_>>>());
    let feat = match json::decode(feat.as_slice()) {
        Ok(map) => map,
        Err(..) => return Err(human("malformed feature header")),
    };

    // Make sure the tarball being uploaded looks sane
    let length = try!(req.content_length().require(|| {
        human("missing header: Content-Length")
    }));
    let max = req.app().config.max_upload_size;
    if length > max { return Err(human(format!("max upload size is: {}", max))) }
    {
        let ty = try!(header(req, "Content-Type"))[0];
        if ty != "application/x-tar" {
            return Err(human(format!("expected `application/x-tar`, \
                                      found `{}`", ty)))
        }
        let enc = try!(header(req, "Content-Encoding"))[0];
        if enc != "gzip" && enc != "x-gzip" {
            return Err(human(format!("expected `gzip`, found `{}`", enc)))
        }
    }

    // Make sure the api token is a valid api token
    let user = try!(User::find_by_api_token(try!(req.tx()),
                                            auth.as_slice()).map_err(|_| {
        human("invalid or unknown auth token supplied")
    }));

    // Validate the name parameter and such
    let new_crate = NewCrate {
        name: name.as_slice().chars().map(|c| c.to_lowercase()).collect(),
        vers: vers,
        deps: deps,
        features: feat,
        cksum: String::new(),
    };
    if !Crate::valid_name(new_crate.name.as_slice()) {
        return Err(human(format!("invalid crate name: `{}`", new_crate.name)))
    }
    if !Version::valid(new_crate.vers.as_slice()) {
        return Err(human(format!("invalid crate version: `{}`", new_crate.vers)))
    }
    Ok((new_crate, user))
}

pub fn download(req: &mut Request) -> CargoResult<Response> {
    let crate_name = req.params()["crate_id"].as_slice();
    let version = req.params()["version"].as_slice();
    let tx = try!(req.tx());
    let stmt = try!(tx.prepare("SELECT crates.id as crate_id,
                                       versions.id as version_id
                                FROM crates
                                LEFT JOIN versions ON
                                    crates.id = versions.crate_id
                                WHERE crates.name = $1
                                  AND versions.num = $2
                                LIMIT 1"));
    let mut rows = try!(stmt.query(&[&crate_name as &ToSql, &version as &ToSql]));
    let row = try!(rows.next().require(|| human("crate or version not found")));
    let crate_id: i32 = row.get("crate_id");
    let version_id: i32 = row.get("version_id");

    // Bump download counts.
    //
    // Note that this is *not* an atomic update, and that's somewhat
    // intentional. It doesn't appear that postgres supports an atomic update of
    // a counter, so we just do the hopefully "least racy" thing. This is
    // largely ok because these download counters are just that, counters. No
    // need to have super high-fidelity counter.
    try!(tx.execute("UPDATE crates SET downloads = downloads + 1
                     WHERE id = $1", &[&crate_id]));
    try!(tx.execute("UPDATE versions SET downloads = downloads + 1
                     WHERE id = $1", &[&version_id]));
    try!(tx.execute("UPDATE metadata SET total_downloads = total_downloads + 1",
                    &[]));

    // Now that we've done our business, redirect to the actual data.
    let redirect_url = format!("https://{}/pkg/{}/{}-{}.tar.gz",
                               req.app().bucket.host(),
                               crate_name, crate_name, version);

    if req.wants_json() {
        #[deriving(Encodable)]
        struct R { ok: bool, url: String }
        Ok(req.json(&R{ ok: true, url: redirect_url }))
    } else {
        Ok(req.redirect(redirect_url))
    }
}
