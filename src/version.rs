use pg::{PostgresConnection, PostgresRow};
use semver;

use app::App;
use db::Connection;
use package::Package;
use util::{RequestUtils, CargoResult, Require, internal};

#[deriving(Clone)]
pub struct Version {
    pub id: i32,
    pub package_id: i32,
    pub num: String,
}

#[deriving(Encodable)]
pub struct EncodableVersion {
    pub id: i32,
    pub pkg: String,
    pub num: String,
    pub url: String,
}

impl Version {
    pub fn from_row(row: &PostgresRow) -> Version {
        Version {
            id: row.get("id"),
            package_id: row.get("package_id"),
            num: row.get("num"),
        }
    }

    pub fn find_by_num(conn: &Connection, package_id: i32, num: &str)
                       -> CargoResult<Option<Version>> {
        let stmt = try!(conn.prepare("SELECT * FROM versions \
                                      WHERE package_id = $1 AND num = $2"));
        let mut rows = try!(stmt.query(&[&package_id, &num]));
        Ok(rows.next().map(|r| Version::from_row(&r)))
    }

    pub fn insert(conn: &Connection, package_id: i32,
                  num: &str) -> CargoResult<Version> {
        let stmt = try!(conn.prepare("INSERT INTO versions (package_id, num) \
                                      VALUES ($1, $2) \
                                      RETURNING *"));
        let mut rows = try!(stmt.query(&[&package_id, &num]));
        Ok(Version::from_row(&try!(rows.next().require(|| {
            internal("no version returned")
        }))))
    }

    pub fn valid(version: &str) -> bool {
        semver::parse(version).is_some()
    }

    pub fn encodable(self, app: &App, pkg: &Package) -> EncodableVersion {
        let Version { id, package_id, num } = self;
        assert_eq!(pkg.id, package_id);
        EncodableVersion {
            url: format!("https://{}{}",
                         app.bucket.host(), pkg.path(num.as_slice())),
            num: num,
            id: id,
            pkg: pkg.name.clone(),
        }
    }
}

pub fn setup(conn: &PostgresConnection) {
    conn.execute("DROP TABLE IF EXISTS versions", []).unwrap();
    conn.execute("CREATE TABLE versions (
                    id              SERIAL PRIMARY KEY,
                    package_id      INTEGER NOT NULL,
                    num             VARCHAR NOT NULL
                  )", []).unwrap();
    conn.execute("ALTER TABLE versions ADD CONSTRAINT \
                  unique_num UNIQUE (package_id, num)", []).unwrap();
}
