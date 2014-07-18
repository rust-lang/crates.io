use std::sync::Arc;

use conduit::{Request, Response};
use conduit_router::RequestParams;
use conduit_json_parser;
use pg::{PostgresConnection, PostgresRow};
use pg::error::PgDbError;
use curl::http;

use app::{App, RequestApp};
use db::Connection;
use git;
use user::{RequestUser, User};
use util::{RequestUtils, CargoResult, Require, internal, ChainError};
use util::errors::{NotFound, CargoError};

pub struct Package {
    pub id: i32,
    pub name: String,
}

#[deriving(Encodable)]
pub struct EncodablePackage {
    pub id: String,
    pub name: String,
}

impl Package {
    fn from_row(row: &PostgresRow) -> Package {
        Package {
            id: row.get("id"),
            name: row.get("name"),
        }
    }

    pub fn find_by_name(conn: &Connection, name: &str) -> CargoResult<Package> {
        let stmt = try!(conn.prepare("SELECT * FROM packages \
                                      WHERE name = $1 LIMIT 1"));
        match try!(stmt.query([&name])).next() {
            Some(row) => Ok(Package::from_row(&row)),
            None => Err(NotFound.box_error()),
        }
    }

    pub fn valid_name(name: &str) -> bool {
        if name.len() == 0 { return false }
        name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }

    fn encodable(self) -> EncodablePackage {
        let Package { name, .. } = self;
        EncodablePackage { id: name.clone(), name: name }
    }
}

pub fn setup(conn: &PostgresConnection) {
    conn.execute("DROP TABLE IF EXISTS packages", []).unwrap();
    conn.execute("CREATE TABLE packages (
                    id              SERIAL PRIMARY KEY,
                    name            VARCHAR NOT NULL
                  )", []).unwrap();

    conn.execute("ALTER TABLE packages ADD CONSTRAINT \
                  unique_name UNIQUE (name)", []).unwrap();
    conn.execute("INSERT INTO packages (name) VALUES ($1)",
                 [&"Test"]).unwrap();
    conn.execute("INSERT INTO packages (name) VALUES ($1)",
                 [&"Test2"]).unwrap();
}

pub fn index(req: &mut Request) -> CargoResult<Response> {
    let limit = 10i64;
    let offset = 0i64;
    let conn = req.app().db();
    let stmt = try!(conn.prepare("SELECT * FROM packages LIMIT $1 OFFSET $2"));

    let mut pkgs = Vec::new();
    for row in try!(stmt.query([&limit, &offset])) {
        pkgs.push(Package::from_row(&row).encodable());
    }

    let stmt = try!(conn.prepare("SELECT COUNT(*) FROM packages"));
    let row = try!(stmt.query([])).next().unwrap();
    let total = row.get(0u);

    #[deriving(Encodable)]
    struct R { packages: Vec<EncodablePackage>, meta: Meta }
    #[deriving(Encodable)]
    struct Meta { total: i64, page: i64 }

    Ok(req.json(&R {
        packages: pkgs,
        meta: Meta { total: total, page: offset / limit }
    }))
}

pub fn show(req: &mut Request) -> CargoResult<Response> {
    let name = &req.params()["package_id"];
    let pkg = try!(Package::find_by_name(&req.app().db(), name.as_slice()));

    #[deriving(Encodable)]
    struct R { package: EncodablePackage }
    Ok(req.json(&R { package: pkg.encodable() }))
}

#[deriving(Decodable)]
pub struct UpdateRequest { package: UpdatePackage }

#[deriving(Decodable)]
pub struct UpdatePackage {
    name: String,
}

pub fn update(req: &mut Request) -> CargoResult<Response> {
    try!(req.user());
    let conn = req.app().db();
    let name = &req.params()["package_id"];
    let pkg = try!(Package::find_by_name(&conn, name.as_slice()));

    let update = conduit_json_parser::json_params::<UpdateRequest>(req).unwrap();
    // TODO: this should do something
    println!("new name: {}", update.package.name);

    #[deriving(Encodable)]
    struct R { package: EncodablePackage }
    Ok(req.json(&R { package: pkg.encodable() }))
}

#[deriving(Encodable)]
pub struct NewPackage {
    pub name: String,
    pub vers: String,
    pub deps: Vec<String>,
}

pub fn new(req: &mut Request) -> CargoResult<Response> {
    let app = req.app().clone();
    let db = app.db();
    let mut tx = try!(db.transaction());
    tx.set_rollback();

    // Peel out all input parameters
    fn header<'a>(req: &'a Request, hdr: &str) -> CargoResult<Vec<&'a str>> {
        req.headers().find(hdr).require(|| {
            internal(format!("missing {} header", hdr))
        })
    }
    let auth = try!(header(req, "X-Cargo-Auth"))[0].to_string();
    let name = try!(header(req, "X-Cargo-Pkg-Name"))[0].to_string();
    let vers = try!(header(req, "X-Cargo-Pkg-Version"))[0].to_string();
    let deps = req.headers().find("X-Cargo-Pkg-Dep").unwrap_or(Vec::new())
                  .move_iter().map(|s| s.to_string()).collect();

    // Make sure the tarball being uploaded looks sane
    let length = try!(req.content_length().require(|| {
        internal("missing Content-Length header")
    }));
    {
        let ty = try!(header(req, "Content-Type"))[0];
        if ty != "application/x-tar" {
            return Err(internal(format!("expected `application/x-tar`, \
                                         found `{}`", ty)))
        }
        let enc = try!(header(req, "Content-Encoding"))[0];
        if enc != "gzip" && enc != "x-gzip" {
            return Err(internal(format!("expected `gzip`, found `{}`", enc)))
        }
    }

    // Make sure the api token is a valid api token
    let _user = try!(User::find_by_api_token(&tx, auth.as_slice()));

    // Validate the name parameter and such
    let name: String = name.as_slice().chars()
                           .map(|c| c.to_lowercase()).collect();
    let new_pkg = NewPackage {
        name: name,
        vers: vers,
        deps: deps,
    };
    if !Package::valid_name(new_pkg.name.as_slice()) {
        return Err(internal(format!("invalid crate name: `{}`", new_pkg.name)))
    }

    // Persist the new package, if it doesn't already exist
    match tx.execute("INSERT INTO packages (name) VALUES ($1)", [&new_pkg.name]) {
        Ok(..) => {}
        Err(PgDbError(ref e))
            if e.constraint.as_ref().map(|a| a.as_slice())
                == Some("unique_name") => {
                tx.set_rollback();
                try!(tx.finish());
                tx = try!(db.transaction());
            }
        Err(e) => return Err(e.box_error()),
    }

    // Upload the package to S3
    let mut handle = http::handle();
    let path = format!("/pkg/{}/{}-{}.tar.gz", new_pkg.name,
                       new_pkg.name, new_pkg.vers);
    let resp = {
        let s3req = app.bucket.put(&mut handle, path.as_slice(), req.body(),
                                   "application/x-tar")
                              .content_length(length)
                              .header("Content-Encoding", "gzip");
        try!(s3req.exec().chain_error(|| {
            internal(format!("failed to upload to S3: `{}`", path))
        }))
    };
    if resp.get_code() != 200 {
        return Err(internal(format!("failed to get a 200 response from S3: {}",
                                    resp)))
    }

    // If the git commands fail below, we shouldn't keep the package on the
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
    let mut bomb = Bomb { app: app, path: Some(path), handle: handle };

    // Register this package in our local git repo.
    let pkg = try!(Package::find_by_name(&tx, new_pkg.name.as_slice()));
    try!(git::add_package(&**req.app(), &new_pkg).chain_error(|| {
        internal(format!("could not add package `{}` to the git repo", pkg.name))
    }));

    // Now that we've come this far, we're committed!
    tx.set_commit();
    bomb.path = None;

    #[deriving(Encodable)]
    struct R { package: EncodablePackage }
    Ok(req.json(&R { package: pkg.encodable() }))
}
