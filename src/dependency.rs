use semver;

use pg::PostgresRow;

use db::Connection;
use util::{CargoResult};
use model::Model;

pub struct Dependency {
    pub id: i32,
    pub version_id: i32,
    pub crate_id: i32,
    pub req: semver::VersionReq,
    pub optional: bool,
    pub default_features: bool,
    pub features: Vec<String>,
}

impl Dependency {
    pub fn insert(conn: &Connection, version_id: i32, crate_id: i32,
                  req: &semver::VersionReq, optional: bool, default_features: bool,
                  features: &[String]) -> CargoResult<Dependency> {
        let req = req.to_string();
        let features = features.connect(",");
        let stmt = try!(conn.prepare("INSERT INTO dependencies
                                      (version_id, crate_id, req, optional,
                                       default, features)
                                      VALUES ($1, $2, $3, $4, $5, $6)
                                      RETURNING *"));
        let mut rows = try!(stmt.query(&[&version_id, &crate_id, &req,
                                         &optional, &default_features,
                                         &features]));
        Ok(Model::from_row(&rows.next().unwrap()))
    }

    pub fn git_encode(&self, crate_name: &str) -> String {
        format!("{}{}{}|{}|{}",
                if self.optional {"-"} else {""},
                if self.default_features {""} else {"*"},
                crate_name,
                self.features.connect(","),
                self.req)
    }
}

impl Model for Dependency {
    fn from_row(row: &PostgresRow) -> Dependency {
        let features: String = row.get("features");
        let req: String = row.get("req");
        Dependency {
            id: row.get("id"),
            version_id: row.get("version_id"),
            crate_id: row.get("crate_id"),
            req: semver::VersionReq::parse(req.as_slice()).unwrap(),
            optional: row.get("optional"),
            default_features: row.get("defaults"),
            features: features.as_slice().split(',').map(|s| s.to_string())
                              .collect(),
        }
    }

    fn table_name(_: Option<Dependency>) -> &'static str { "dependencies" }
}
