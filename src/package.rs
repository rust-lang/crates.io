use std::any::AnyRefExt;
use std::io::{IoResult, MemReader};
use std::rand::{task_rng, Rng};
use std::str;
use std::collections::HashMap;
use serialize::json;

use conduit::{Request, Response};
use conduit_cookie::{RequestSession};
use conduit_router::RequestParams;
use curl::http;
use oauth2::Authorization;
use pg::{PostgresConnection, PostgresRow};
use pg::error::PgDbError;

use app::{App, RequestApp};
use util::{RequestJson, RequestQuery};

#[deriving(Encodable)]
pub struct Package {
    pub id: String,
    pub name: String,
}

impl Package {
    fn from_row(row: &PostgresRow) -> Package {
        Package {
            id: row["slug"],
            name: row["name"],
        }
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
    let total = row[0u];

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
        None => return Ok(Response {
            status: (404, "Not Found"),
            headers: HashMap::new(),
            body: box MemReader::new(Vec::new()),
        }),
    };

    #[deriving(Encodable)]
    struct R { package: Package }

    let pkg = Package::from_row(&row);
    Ok(req.json(&R { package: pkg }))
}
