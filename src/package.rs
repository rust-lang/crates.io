use conduit::{Request, Response};
use conduit_router::RequestParams;
use conduit_json_parser;
use pg::{PostgresConnection, PostgresRow};

use app::RequestApp;
use db::Connection;
use git;
use user::{RequestUser, User};
use util::{RequestUtils, CargoResult, Require, internal, ChainError};
use util::errors::{NotFound, CargoError};

#[deriving(Encodable)]
pub struct Package {
    pub id: String,
    pub name: String,
}

impl Package {
    fn from_row(row: &PostgresRow) -> Package {
        Package {
            id: row.get("slug"),
            name: row.get("name"),
        }
    }

    pub fn find(conn: &Connection, slug: &str) -> CargoResult<Package> {
        let stmt = try!(conn.prepare("SELECT * FROM packages \
                                      WHERE slug = $1 LIMIT 1"));
        match try!(stmt.query([&slug])).next() {
            Some(row) => Ok(Package::from_row(&row)),
            None => Err(NotFound.box_error()),
        }
    }

    fn name_to_slug(name: &str) -> String {
        name.chars().filter_map(|c| {
            match c {
                'A' .. 'Z' |
                'a' .. 'z' |
                '0' .. '9' |
                '-' | '_' => Some(c.to_lowercase()),
                _ => None

            }
        }).collect()
    }
}

pub fn setup(conn: &PostgresConnection) {
    conn.execute("DROP TABLE IF EXISTS packages", []).unwrap();
    conn.execute("CREATE TABLE packages (
                    id              SERIAL PRIMARY KEY,
                    name            VARCHAR NOT NULL,
                    slug            VARCHAR NOT NULL
                  )", []).unwrap();

    conn.execute("ALTER TABLE packages ADD CONSTRAINT \
                  unique_slug UNIQUE (slug)", []).unwrap();
    conn.execute("INSERT INTO packages (name, slug) VALUES ($1, $2)",
                 [&"Test", &"test"]).unwrap();
    conn.execute("INSERT INTO packages (name, slug) VALUES ($1, $2)",
                 [&"Test2", &"test2"]).unwrap();
}

pub fn index(req: &mut Request) -> CargoResult<Response> {
    let limit = 10i64;
    let offset = 0i64;
    let conn = req.app().db();
    let stmt = try!(conn.prepare("SELECT * FROM packages LIMIT $1 OFFSET $2"));

    let mut pkgs = Vec::new();
    for row in try!(stmt.query([&limit, &offset])) {
        pkgs.push(Package::from_row(&row));
    }

    let stmt = try!(conn.prepare("SELECT COUNT(*) FROM packages"));
    let row = try!(stmt.query([])).next().unwrap();
    let total = row.get(0u);

    #[deriving(Encodable)]
    struct R { packages: Vec<Package>, meta: Meta }
    #[deriving(Encodable)]
    struct Meta { total: i64, page: i64 }

    Ok(req.json(&R {
        packages: pkgs,
        meta: Meta { total: total, page: offset / limit }
    }))
}

pub fn show(req: &mut Request) -> CargoResult<Response> {
    let slug = req.params()["package_id"];
    let pkg = try!(Package::find(&req.app().db(), slug.as_slice()));

    #[deriving(Encodable)]
    struct R { package: Package }
    Ok(req.json(&R { package: pkg }))
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
    let slug = req.params()["package_id"];
    let mut pkg = try!(Package::find(&conn, slug.as_slice()));

    let update = conduit_json_parser::json_params::<UpdateRequest>(req);
    pkg.name = update.unwrap().package.name.clone();
    try!(conn.execute("UPDATE packages SET name = $1 WHERE slug = $2",
                      [&pkg.name.as_slice(), &slug.as_slice()]));

    #[deriving(Encodable)]
    struct R { package: Package }
    Ok(req.json(&R { package: pkg }))
}

#[deriving(Decodable)]
pub struct NewRequest { package: NewPackage }

#[deriving(Decodable)]
pub struct NewPackage {
    name: String,
}

pub fn new(req: &mut Request) -> CargoResult<Response> {
    let app = req.app();
    let db = app.db();
    let tx = try!(db.transaction());
    tx.set_rollback();
    let _user = {
        let header = try!(req.headers().find("X-Cargo-Auth").require(|| {
            internal("missing X-Cargo-Auth header")
        }));
        try!(User::find_by_api_token(&tx, header.get(0).as_slice()))
    };

    let update = conduit_json_parser::json_params::<NewRequest>(req).unwrap();
    let name = update.package.name.as_slice();
    let slug = Package::name_to_slug(name);
    try!(tx.execute("INSERT INTO packages (name, slug) VALUES ($1, $2)",
                    [&name, &slug]));

    #[deriving(Encodable)]
    struct R { package: Package }
    let pkg = try!(Package::find(&tx, slug.as_slice()));
    try!(git::add_package(app, &pkg).chain_error(|| {
        internal(format!("could not add package `{}` to the git repo", pkg.name))
    }));
    tx.set_commit();
    Ok(req.json(&R { package: pkg }))
}
