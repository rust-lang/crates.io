use conduit::{Request, Response};
use conduit_router::RequestParams;
use conduit_json_parser;
use pg::{PostgresConnection, PostgresRow};
use pg::error::PgDbError;

use app::RequestApp;
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

#[deriving(Decodable, Clone)]
pub struct NewRequest { package: NewPackage }

#[deriving(Decodable, Encodable, Clone)]
pub struct NewPackage {
    pub name: String,
    pub version: String,
    pub other: String,
    pub dependencies: Vec<String>,
}

pub fn new(req: &mut Request) -> CargoResult<Response> {
    let app = req.app();
    let db = app.db();
    let mut tx = try!(db.transaction());
    tx.set_rollback();
    let _user = {
        let header = try!(req.headers().find("X-Cargo-Auth").require(|| {
            internal("missing X-Cargo-Auth header")
        }));
        try!(User::find_by_api_token(&tx, header[0].as_slice()))
    };

    let new = conduit_json_parser::json_params::<NewRequest>(req).unwrap();
    let mut new = new.clone();
    new.package.name = new.package.name.as_slice().chars()
                          .map(|c| c.to_lowercase()).collect();
    let name = new.package.name.as_slice();
    if !Package::valid_name(name.as_slice()) {
        return Err(internal(format!("invalid crate name: `{}`", name)))
    }
    match tx.execute("INSERT INTO packages (name) VALUES ($1)", [&name]) {
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

    let pkg = try!(Package::find_by_name(&tx, name.as_slice()));
    try!(git::add_package(app, &new.package).chain_error(|| {
        internal(format!("could not add package `{}` to the git repo", pkg.name))
    }));
    tx.set_commit();

    #[deriving(Encodable)]
    struct R { package: EncodablePackage }
    Ok(req.json(&R { package: pkg.encodable() }))
}
