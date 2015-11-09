use pg::GenericConnection;
use pg::rows::Row;
use semver;

use Model;
use git;
use util::{CargoResult};

pub struct Dependency {
    pub id: i32,
    pub version_id: i32,
    pub crate_id: i32,
    pub req: semver::VersionReq,
    pub optional: bool,
    pub default_features: bool,
    pub features: Vec<String>,
    pub target: Option<String>,
    pub kind: Kind,
}

#[derive(RustcEncodable, RustcDecodable)]
pub struct EncodableDependency {
    pub id: i32,
    pub version_id: i32,
    pub crate_id: String,
    pub req: String,
    pub optional: bool,
    pub default_features: bool,
    pub features: String,
    pub target: Option<String>,
    pub kind: Kind,
}

#[derive(Copy, Clone)]
// NB: this order is important, it must be retained! The database stores an
// integer corresponding to each variant.
pub enum Kind {
    Normal,
    Build,
    Dev,

    // if you add a kind here, be sure to update `from_row` below.
}

impl Dependency {
    pub fn insert(conn: &GenericConnection, version_id: i32, crate_id: i32,
                  req: &semver::VersionReq, kind: Kind,
                  optional: bool, default_features: bool,
                  features: &[String], target: &Option<String>)
                  -> CargoResult<Dependency> {
        let req = req.to_string();
        let features = features.join(",");
        let stmt = try!(conn.prepare("INSERT INTO dependencies
                                      (version_id, crate_id, req, optional,
                                       default_features, features, target, kind)
                                      VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                                      RETURNING *"));
        let rows = try!(stmt.query(&[&version_id, &crate_id, &req,
                                      &optional, &default_features,
                                      &features, target, &(kind as i32)]));
        Ok(Model::from_row(&rows.iter().next().unwrap()))
    }

    pub fn git_encode(&self, crate_name: &str) -> git::Dependency {
        let Dependency { id: _, version_id: _, crate_id: _, ref req,
                         optional, default_features, ref features,
                         ref target, kind } = *self;
        git::Dependency {
            name: crate_name.to_string(),
            req: req.to_string(),
            features: features.clone(),
            optional: optional,
            default_features: default_features,
            target: target.clone(),
            kind: Some(kind),
        }
    }

    pub fn encodable(self, crate_name: &str) -> EncodableDependency {
        let Dependency { id, version_id, crate_id: _, req, optional,
                         default_features, features, target, kind } = self;
        EncodableDependency {
            id: id,
            version_id: version_id,
            crate_id: crate_name.to_string(),
            req: req.to_string(),
            optional: optional,
            default_features: default_features,
            features: features.join(","),
            target: target,
            kind: kind,
        }
    }
}

impl Model for Dependency {
    fn from_row(row: &Row) -> Dependency {
        let features: String = row.get("features");
        let req: String = row.get("req");
        let kind: Option<i32> = row.get("kind");
        Dependency {
            id: row.get("id"),
            version_id: row.get("version_id"),
            crate_id: row.get("crate_id"),
            req: semver::VersionReq::parse(&req).unwrap(),
            optional: row.get("optional"),
            default_features: row.get("default_features"),
            features: features.split(',').filter(|s| !s.is_empty())
                              .map(|s| s.to_string())
                              .collect(),
            target: row.get("target"),
            kind: match kind.unwrap_or(0) {
                0 => Kind::Normal,
                1 => Kind::Build,
                2 => Kind::Dev,
                n => panic!("unknown kind: {}", n),
            }
        }
    }

    fn table_name(_: Option<Dependency>) -> &'static str { "dependencies" }
}
