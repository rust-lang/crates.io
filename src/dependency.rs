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
    pub features: Vec<String>,
    pub target: Option<String>,
    pub kind: Kind,
    pub downloads: i32,
}

#[derive(Copy, Clone)]
#[repr(u32)]
pub enum Kind {
    Normal = 0,
    Build = 1,
    Dev = 2,
    // if you add a kind here, be sure to update `from_row` below.
}

impl Dependency {
    pub fn insert(conn: &GenericConnection, version_id: i32, crate_id: i32,
                  req: &semver::VersionReq, kind: Kind,
                  optional: bool, default_features: bool,
                  features: &[String], target: &Option<String>)
                  -> CargoResult<Dependency> {
        let req = req.to_string();
        let stmt = conn.prepare("INSERT INTO dependencies
                                      (version_id, crate_id, req, optional,
                                       default_features, features, target, kind)
                                      VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                                      RETURNING *")?;
        let rows = stmt.query(&[&version_id, &crate_id, &req,
            &optional, &default_features,
            &features, target, &(kind as i32)])?;
        Ok(Model::from_row(&rows.iter().next().unwrap()))
    }

    pub fn git_encode(self, crate_name: &str) -> git::Dependency {
        git::Dependency {
            name: crate_name.into(),
            req: self.req.to_string(),
            features: self.features,
            optional: self.optional,
            default_features: self.default_features,
            target: self.target,
            kind: Some(self.kind),
        }
    }

    // `downloads` need only be specified when generating a reverse dependency
    pub fn encodable(self, crate_name: &str, downloads: Option<i32>) -> EncodableDependency {
        EncodableDependency {
            id: self.id,
            version_id: self.version_id,
            crate_id: crate_name.into(),
            req: self.req.to_string(),
            optional: self.optional,
            default_features: self.default_features,
            features: self.features,
            target: self.target,
            kind: self.kind,
            downloads: downloads.unwrap_or(0),
        }
    }
}

impl Model for Dependency {
    fn from_row(row: &Row) -> Dependency {
        let req: String = row.get("req");
        Dependency {
            id: row.get("id"),
            version_id: row.get("version_id"),
            crate_id: row.get("crate_id"),
            req: semver::VersionReq::parse(&req).unwrap(),
            optional: row.get("optional"),
            default_features: row.get("default_features"),
            features: row.get("features"),
            target: row.get("target"),
            kind: match row.get("kind") {
                0 => Kind::Normal,
                1 => Kind::Build,
                2 => Kind::Dev,
                n => panic!("unknown kind: {}", n),
            }
        }
    }

    fn table_name(_: Option<Dependency>) -> &'static str { "dependencies" }
}
