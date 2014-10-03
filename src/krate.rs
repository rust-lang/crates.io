use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use serialize::hex::ToHex;
use serialize::json;
use time::Timespec;

use conduit::{Request, Response};
use conduit_router::RequestParams;
use curl::http;
use pg::types::ToSql;
use pg::{PostgresRow, PostgresStatement};
use semver;

use app::{App, RequestApp};
use db::{Connection, RequestTransaction};
use download::{VersionDownload, EncodableVersionDownload};
use git;
use model::Model;
use upload;
use user::{User, RequestUser};
use util::errors::{NotFound, CargoError};
use util::{LimitErrorReader, HashingReader};
use util::{RequestUtils, CargoResult, Require, internal, ChainError, human};
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
    pub updated_at: String,
    pub versions: Option<Vec<i32>>,
    pub created_at: String,
    pub downloads: i32,
    pub max_version: String,
    pub links: CrateLinks,
}

#[deriving(Encodable, Decodable)]
pub struct CrateLinks {
    pub version_downloads: String,
    pub versions: Option<String>,
}

impl Crate {
    pub fn find(conn: &Connection, id: i32) -> CargoResult<Crate> {
        Model::find(conn, id)
    }

    pub fn find_by_name(conn: &Connection, name: &str) -> CargoResult<Crate> {
        let stmt = try!(conn.prepare("SELECT * FROM crates \
                                      WHERE name = $1 LIMIT 1"));
        match try!(stmt.query(&[&name as &ToSql])).next() {
            Some(row) => Ok(Model::from_row(&row)),
            None => Err(NotFound.box_error()),
        }
    }

    pub fn find_or_insert(conn: &Connection, name: &str,
                          user_id: i32) -> CargoResult<Crate> {
        // TODO: like with users, this is sadly racy

        let stmt = try!(conn.prepare("SELECT * FROM crates WHERE name = $1"));
        let mut rows = try!(stmt.query(&[&name as &ToSql]));
        match rows.next() {
            Some(row) => return Ok(Model::from_row(&row)),
            None => {}
        }
        let stmt = try!(conn.prepare("INSERT INTO crates \
                                      (name, user_id, created_at,
                                       updated_at, downloads, max_version) \
                                      VALUES ($1, $2, $3, $4, 0, '0.0.0') \
                                      RETURNING *"));
        let now = ::now();
        let mut rows = try!(stmt.query(&[&name as &ToSql, &user_id, &now, &now]));
        Ok(Model::from_row(&try!(rows.next().require(|| {
            internal("no crate returned")
        }))))
    }

    pub fn valid_name(name: &str) -> bool {
        if name.len() == 0 { return false }
        name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }

    pub fn encodable(self, versions: Option<Vec<i32>>) -> EncodableCrate {
        let Crate { name, created_at, updated_at, downloads,
                    max_version, .. } = self;
        let versions_link = match versions {
            Some(..) => None,
            None => Some(format!("/crates/{}/versions", name)),
        };
        EncodableCrate {
            id: name.clone(),
            name: name.clone(),
            updated_at: ::encode_time(updated_at),
            created_at: ::encode_time(created_at),
            downloads: downloads,
            versions: versions,
            max_version: max_version.to_string(),
            links: CrateLinks {
                version_downloads: format!("/crates/{}/downloads", name),
                versions: versions_link,
            },
        }
    }

    pub fn versions(&self, conn: &Connection) -> CargoResult<Vec<Version>> {
        let stmt = try!(conn.prepare("SELECT * FROM versions \
                                      WHERE crate_id = $1"));
        let rows = try!(stmt.query(&[&self.id]));
        Ok(rows.map(|r| Model::from_row(&r)).collect())
    }

    pub fn s3_path(&self, version: &str) -> String {
        format!("/pkg/{}/{}-{}.tar.gz", self.name, self.name, version)
    }

    pub fn add_version(&mut self, conn: &Connection, ver: &semver::Version,
                       features: &HashMap<String, Vec<String>>)
                       -> CargoResult<Version> {
        match try!(Version::find_by_num(conn, self.id, ver)) {
            Some(..) => {
                return Err(human(format!("crate version `{}` is already uploaded",
                                         ver)))
            }
            None => {}
        }
        if *ver > self.max_version { self.max_version = ver.clone(); }
        self.updated_at = ::now();
        try!(conn.execute("UPDATE crates SET updated_at = $1, max_version = $2
                           WHERE id = $3",
                          &[&self.updated_at, &self.max_version.to_string(),
                            &self.id]));
        Version::insert(conn, self.id, ver, features)
    }
}

impl Model for Crate {
    fn from_row(row: &PostgresRow) -> Crate {
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
    fn table_name(_: Option<Crate>) -> &'static str { "crates" }
}

pub fn index(req: &mut Request) -> CargoResult<Response> {
    let conn = try!(req.tx());
    let (offset, limit) = try!(req.pagination(10, 100));
    let query = req.query();
    let sort = query.find_equiv(&"sort").map(|s| s.as_slice()).unwrap_or("alpha");
    let sort_sql = match sort {
        "downloads" => "ORDER BY crates.downloads DESC",
        _ => "ORDER BY crates.name ASC",
    };

    // Different queries for different parameters
    let mut pattern;
    let mut id;
    let mut args = vec![&limit as &ToSql, &offset as &ToSql];
    let (q, cnt) = match (query.find_equiv(&"q"), query.find_equiv(&"letter")) {
        (Some(query), _) => {
            args.insert(0, query as &ToSql);
            ("SELECT crates.* FROM crates,
                                   plainto_tsquery($1) q,
                                   to_tsvector('english', name) txt,
                                   ts_rank_cd(txt, q) rank
              WHERE q @@ txt
              ORDER BY rank DESC LIMIT $2 OFFSET $3".to_string(),
             "SELECT COUNT(crates.*) FROM crates,
                                          plainto_tsquery($1) q,
                                          to_tsvector('english', name) txt
              WHERE q @@ txt")
        }
        (None, Some(letter)) => {
            pattern = format!("{}%", letter.as_slice().char_at(0)
                                           .to_lowercase());
            args.insert(0, &pattern as &ToSql);
            (format!("SELECT * FROM crates WHERE name LIKE $1 {}
                      LIMIT $2 OFFSET $3", sort_sql),
             "SELECT COUNT(*) FROM crates WHERE name LIKE $1")
        },
        (None, None) => {
            let user_id = query.find_equiv(&"user_id").map(|s| s.as_slice())
                               .and_then(from_str::<i32>);
            let following = query.find_equiv(&"following").is_some();
            match (user_id, following) {
                (Some(user_id), _) => {
                    id = user_id;
                    args.insert(0, &id as &ToSql);
                    (format!("SELECT * FROM crates WHERE user_id = $1 {} \
                              LIMIT $2 OFFSET $3",
                             sort_sql),
                     "SELECT COUNT(*) FROM crates WHERE user_id = $1")
                }
                (None, true) => {
                    let me = try!(req.user());
                    id = me.id;
                    args.insert(0, &id as &ToSql);
                    (format!("SELECT crates.* FROM crates
                              INNER JOIN follows
                                 ON follows.crate_id = crates.id AND
                                    follows.user_id = $1
                              {} LIMIT $2 OFFSET $3", sort_sql),
                     "SELECT COUNT(crates.*) FROM crates
                      INNER JOIN follows
                         ON follows.crate_id = crates.id AND
                            follows.user_id = $1")
                }
                (None, false) => {
                    (format!("SELECT * FROM crates {} LIMIT $1 OFFSET $2",
                             sort_sql),
                     "SELECT COUNT(*) FROM crates")
                }
            }
        }
    };

    // Collect all the crates
    let stmt = try!(conn.prepare(q.as_slice()));
    let mut crates = Vec::new();
    for row in try!(stmt.query(args.as_slice())) {
        let krate: Crate = Model::from_row(&row);
        crates.push(krate.encodable(None));
    }

    // Query for the total count of crates
    let stmt = try!(conn.prepare(cnt));
    let args = if args.len() > 2 {args.slice_to(1)} else {args.slice_to(0)};
    let row = try!(stmt.query(args)).next().unwrap();
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
        let rows = raw_try!(stmt.query([]));
        Ok(rows.map(|r| {
            let krate: Crate = Model::from_row(&r);
            krate.encodable(None)
        }).collect::<Vec<EncodableCrate>>())
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
    let ids = versions.iter().map(|v| v.id).collect();

    #[deriving(Encodable)]
    struct R { krate: EncodableCrate, versions: Vec<EncodableVersion>, }
    Ok(req.json(&R {
        krate: krate.clone().encodable(Some(ids)),
        versions: versions.into_iter().map(|v| {
            v.encodable(krate.name.as_slice())
        }).collect(),
    }))
}

pub fn new(req: &mut Request) -> CargoResult<Response> {
    let app = req.app().clone();

    let (new_crate, user) = try!(parse_new_headers(req));
    let name = new_crate.name.as_slice();
    let vers = &*new_crate.vers;
    let features = new_crate.features.iter().map(|(k, v)| {
        ((**k).to_string(), v.iter().map(|v| (**v).to_string()).collect())
    }).collect::<HashMap<String, Vec<String>>>();

    // Persist the new crate, if it doesn't already exist
    let mut krate = try!(Crate::find_or_insert(try!(req.tx()), name, user.id));
    if krate.user_id != user.id {
        return Err(human("crate name has already been claimed by another user"))
    }

    // Persist the new version of this crate
    let mut version = try!(krate.add_version(try!(req.tx()), vers, &features));

    // Link this new version to all dependencies
    let mut deps = Vec::new();
    for dep in new_crate.deps.iter() {
        let (dep, krate) = try!(version.add_dependency(try!(req.tx()), dep));
        deps.push(dep.git_encode(krate.name.as_slice()));
    }

    // Upload the crate to S3
    let handle = http::handle();
    let mut handle = match req.app().s3_proxy {
        Some(ref proxy) => handle.proxy(proxy.as_slice()),
        None => handle,
    };
    let path = krate.s3_path(vers.to_string().as_slice());
    let (resp, cksum) = {
        let length = try!(req.body().read_le_u32());
        let body = LimitErrorReader::new(req.body(), app.config.max_upload_size);
        let mut body = HashingReader::new(body);
        let resp = {
            let s3req = app.bucket.put(&mut handle, path.as_slice(), &mut body,
                                       "application/x-tar")
                                  .content_length(length as uint)
                                  .header("Content-Encoding", "gzip");
            try!(s3req.exec().chain_error(|| {
                internal(format!("failed to upload to S3: `{}`", path))
            }))
        };
        (resp, body.final())
    };
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
    let git_crate = git::GitCrate {
        name: name.to_string(),
        vers: vers.to_string(),
        cksum: cksum.as_slice().to_hex(),
        features: features,
        deps: deps,
    };
    try!(git::add_crate(&**req.app(), &git_crate).chain_error(|| {
        internal(format!("could not add crate `{}` to the git repo", name))
    }));

    // Now that we've come this far, we're committed!
    bomb.path = None;

    #[deriving(Encodable)]
    struct R { krate: EncodableCrate }
    Ok(req.json(&R { krate: krate.encodable(None) }))
}

fn parse_new_headers(req: &mut Request) -> CargoResult<(upload::NewCrate, User)> {
    // Make sure the tarball being uploaded looks sane
    let length = try!(req.content_length().require(|| {
        human("missing header: Content-Length")
    }));
    let max = req.app().config.max_upload_size;
    if length > max { return Err(human(format!("max upload size is: {}", max))) }

    // Read the json upload request
    let amt = try!(req.body().read_le_u32()) as uint;
    if amt > max { return Err(human(format!("max upload size is: {}", max))) }
    let json = try!(req.body().read_exact(amt));
    let json = try!(String::from_utf8(json).map_err(|_| {
        human("json body was not valid utf-8")
    }));
    let new: upload::NewCrate = try!(json::decode(json.as_slice()).map_err(|e| {
        human(format!("invalid upload request: {}", e))
    }));

    // Peel out authentication
    fn header<'a>(req: &'a Request, hdr: &str) -> CargoResult<Vec<&'a str>> {
        req.headers().find(hdr).require(|| {
            human(format!("missing header: {}", hdr))
        })
    }
    let auth = try!(header(req, "X-Cargo-Auth"))[0].to_string();

    // Make sure the api token is a valid api token
    let user = try!(User::find_by_api_token(try!(req.tx()),
                                            auth.as_slice()).map_err(|_| {
        human("invalid or unknown auth token supplied")
    }));

    Ok((new, user))
}

pub fn download(req: &mut Request) -> CargoResult<Response> {
    let crate_name = req.params()["crate_id"].as_slice();
    let version = req.params()["version"].as_slice();
    let tx = try!(req.tx());
    let stmt = try!(tx.prepare("SELECT versions.id as version_id
                                FROM crates
                                LEFT JOIN versions ON
                                    crates.id = versions.crate_id
                                WHERE crates.name = $1
                                  AND versions.num = $2
                                LIMIT 1"));
    let mut rows = try!(stmt.query(&[&crate_name as &ToSql, &version as &ToSql]));
    let row = try!(rows.next().require(|| human("crate or version not found")));
    let version_id: i32 = row.get("version_id");
    let now = ::now();

    // Bump download counts.
    //
    // Note that this is *not* an atomic update, and that's somewhat
    // intentional. It doesn't appear that postgres supports an atomic update of
    // a counter, so we just do the hopefully "least racy" thing. This is
    // largely ok because these download counters are just that, counters. No
    // need to have super high-fidelity counter.
    //
    // Also, we only update the counter for *today*, nothing else. We have lots
    // of other counters, but they're all updated later on via the
    // update-downloads script.
    let amt = try!(tx.execute("UPDATE version_downloads
                               SET downloads = downloads + 1
                               WHERE version_id = $1 AND date($2) = date(date)",
                              &[&version_id, &now]));
    if amt == 0 {
        try!(tx.execute("INSERT INTO version_downloads
                         (version_id, downloads, counted, date, processed)
                         VALUES ($1, 1, 0, date($2), false)",
                        &[&version_id, &now]));
    }

    // Now that we've done our business, redirect to the actual data.
    let redirect_url = format!("https://{}/pkg/{}/{}-{}.tar.gz",
                               req.app().bucket.host(),
                               crate_name, crate_name, version);

    if req.wants_json() {
        #[deriving(Encodable)]
        struct R { url: String }
        Ok(req.json(&R{ url: redirect_url }))
    } else {
        Ok(req.redirect(redirect_url))
    }
}

pub fn downloads(req: &mut Request) -> CargoResult<Response> {
    let crate_name = req.params()["crate_id"].as_slice();
    let tx = try!(req.tx());
    let krate = try!(Crate::find_by_name(tx, crate_name));

    let cutoff_date = ::now() + Duration::days(-90);
    let stmt = try!(tx.prepare("SELECT * FROM version_downloads
                                LEFT JOIN versions
                                    ON versions.id = version_downloads.version_id
                                WHERE date > $1 AND versions.crate_id = $2
                                ORDER BY date ASC"));
    let mut downloads = Vec::new();
    for row in try!(stmt.query(&[&cutoff_date, &krate.id])) {
        let download: VersionDownload = Model::from_row(&row);
        downloads.push(download.encodable());
    }

    #[deriving(Encodable)]
    struct R { version_downloads: Vec<EncodableVersionDownload> }
    Ok(req.json(&R{ version_downloads: downloads }))
}

fn user_and_crate(req: &mut Request) -> CargoResult<(User, Crate)> {
    let user = try!(req.user());
    let crate_name = req.params()["crate_id"].as_slice();
    let tx = try!(req.tx());
    let krate = try!(Crate::find_by_name(tx, crate_name));
    Ok((user.clone(), krate))
}

pub fn follow(req: &mut Request) -> CargoResult<Response> {
    let (user, krate) = try!(user_and_crate(req));
    let tx = try!(req.tx());
    let stmt = try!(tx.prepare("SELECT 1 FROM follows
                                WHERE user_id = $1 AND crate_id = $2"));
    let mut rows = try!(stmt.query(&[&user.id, &krate.id]));
    if !rows.next().is_some() {
        try!(tx.execute("INSERT INTO follows (user_id, crate_id)
                         VALUES ($1, $2)", &[&user.id, &krate.id]));
    }
    #[deriving(Encodable)]
    struct R { ok: bool }
    Ok(req.json(&R { ok: true }))
}

pub fn unfollow(req: &mut Request) -> CargoResult<Response> {
    let (user, krate) = try!(user_and_crate(req));
    let tx = try!(req.tx());
    try!(tx.execute("DELETE FROM follows
                     WHERE user_id = $1 AND crate_id = $2",
                    &[&user.id, &krate.id]));
    #[deriving(Encodable)]
    struct R { ok: bool }
    Ok(req.json(&R { ok: true }))
}

pub fn following(req: &mut Request) -> CargoResult<Response> {
    let (user, krate) = try!(user_and_crate(req));
    let tx = try!(req.tx());
    let stmt = try!(tx.prepare("SELECT 1 FROM follows
                                WHERE user_id = $1 AND crate_id = $2"));
    let mut rows = try!(stmt.query(&[&user.id, &krate.id]));
    #[deriving(Encodable)]
    struct R { following: bool }
    Ok(req.json(&R { following: rows.next().is_some() }))
}

pub fn versions(req: &mut Request) -> CargoResult<Response> {
    let crate_name = req.params()["crate_id"].as_slice();
    let tx = try!(req.tx());
    let krate = try!(Crate::find_by_name(tx, crate_name));

    let stmt = try!(tx.prepare("SELECT * FROM versions WHERE crate_id = $1"));
    let mut versions = Vec::new();
    for row in try!(stmt.query(&[&krate.id])) {
        let version: Version = Model::from_row(&row);
        versions.push(version.encodable(crate_name));
    }

    #[deriving(Encodable)]
    struct R { versions: Vec<EncodableVersion> }
    Ok(req.json(&R{ versions: versions }))
}
