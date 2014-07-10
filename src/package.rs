use std::io::IoResult;

use conduit::{Request, Response};
use conduit_router::RequestParams;
use conduit_json_parser;
use pg::{PostgresConnection, PostgresRow};

use app::{App, RequestApp};
use user::RequestUser;
use util::RequestUtils;

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

    pub fn find(app: &App, slug: &str) -> Option<Package> {
        let conn = app.db();
        let stmt = conn.prepare("SELECT * FROM packages WHERE slug = $1 LIMIT 1")
                       .unwrap();
        stmt.query([&slug]).unwrap().next().map(|row| {
            Package {
                id: row.get("slug"),
                name: row.get("name"),
            }
        })
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

pub fn index(req: &mut Request) -> IoResult<Response> {
    let limit = 10i64;
    let offset = 0i64;
    let conn = req.app().db();
    let stmt = conn.prepare("SELECT * FROM packages LIMIT $1 OFFSET $2")
                   .unwrap();

    let mut pkgs = Vec::new();
    for row in stmt.query([&limit, &offset]).unwrap() {
        pkgs.push(Package::from_row(&row));
    }

    let stmt = conn.prepare("SELECT COUNT(*) FROM packages").unwrap();
    let row = stmt.query([]).unwrap().next().unwrap();
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

pub fn show(req: &mut Request) -> IoResult<Response> {
    let slug = req.params()["package_id"];
    let conn = req.app().db();
    let stmt = conn.prepare("SELECT * FROM packages WHERE slug = $1 LIMIT 1")
                   .unwrap();
    let row = match stmt.query([&slug.as_slice()]).unwrap().next() {
        Some(row) => row,
        None => return Ok(req.not_found()),
    };

    #[deriving(Encodable)]
    struct R { package: Package }

    let pkg = Package::from_row(&row);
    Ok(req.json(&R { package: pkg }))
}

#[deriving(Decodable)]
pub struct UpdateRequest { package: UpdatePackage }

#[deriving(Decodable)]
pub struct UpdatePackage {
    name: String,
}

pub fn update(req: &mut Request) -> IoResult<Response> {
    if req.user().is_none() {
        return Ok(req.unauthorized())
    }
    let slug = req.params()["package_id"];
    let mut pkg = match Package::find(req.app(), slug.as_slice()) {
        Some(pkg) => pkg,
        None => return Ok(req.not_found()),
    };
    {
        let conn = req.app().db();
        let update = conduit_json_parser::json_params::<UpdateRequest>(req);
        pkg.name = update.unwrap().package.name.clone();
        conn.execute("UPDATE packages SET name = $1 WHERE slug = $2",
                     [&pkg.name.as_slice(), &slug.as_slice()])
            .unwrap();
    }

    #[deriving(Encodable)]
    struct R { package: Package }
    Ok(req.json(&R { package: pkg }))
}
