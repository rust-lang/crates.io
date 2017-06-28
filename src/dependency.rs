use diesel::prelude::*;
use diesel::pg::{Pg, PgConnection};
use diesel::types::{Integer, Text};
use pg::GenericConnection;
use pg::rows::Row;
use semver;

use Model;
use git;
use krate::Crate;
use schema::*;
use util::{CargoResult, human};
use version::Version;

#[derive(Identifiable, Associations, Debug)]
#[belongs_to(Version)]
#[belongs_to(Crate)]
#[table_name = "dependencies"]
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

#[derive(Debug)]
pub struct ReverseDependency {
    dependency: Dependency,
    crate_name: String,
    crate_downloads: i32,
}

#[derive(RustcEncodable, RustcDecodable, Debug)]
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

#[derive(Copy, Clone, Debug)]
#[repr(u32)]
pub enum Kind {
    Normal = 0,
    Build = 1,
    Dev = 2,
    // if you add a kind here, be sure to update `from_row` below.
}

#[derive(Default, Insertable, Debug)]
#[table_name = "dependencies"]
pub struct NewDependency<'a> {
    pub version_id: i32,
    pub crate_id: i32,
    pub req: String,
    pub optional: bool,
    pub default_features: bool,
    pub features: Vec<&'a str>,
    pub target: Option<&'a str>,
    pub kind: i32,
}

impl Dependency {
    // FIXME: Encapsulate this in a `NewDependency` struct
    #[cfg_attr(feature = "clippy", allow(too_many_arguments))]
    pub fn insert(
        conn: &GenericConnection,
        version_id: i32,
        crate_id: i32,
        req: &semver::VersionReq,
        kind: Kind,
        optional: bool,
        default_features: bool,
        features: &[String],
        target: &Option<String>,
    ) -> CargoResult<Dependency> {
        let req = req.to_string();
        let stmt = conn.prepare(
            "INSERT INTO dependencies
                                      (version_id, crate_id, req, optional,
                                       default_features, features, target, kind)
                                      VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                                      RETURNING *",
        )?;
        let rows = stmt.query(
            &[
                &version_id,
                &crate_id,
                &req,
                &optional,
                &default_features,
                &features,
                target,
                &(kind as i32),
            ],
        )?;
        Ok(Model::from_row(&rows.iter().next().unwrap()))
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

impl ReverseDependency {
    pub fn encodable(self) -> EncodableDependency {
        self.dependency.encodable(
            &self.crate_name,
            Some(self.crate_downloads),
        )
    }
}

pub fn add_dependencies(
    conn: &PgConnection,
    deps: &[::upload::CrateDependency],
    version_id: i32,
) -> CargoResult<Vec<git::Dependency>> {
    use diesel::insert;

    let git_and_new_dependencies = deps.iter()
        .map(|dep| {
            let krate = Crate::by_name(&dep.name).first::<Crate>(&*conn).map_err(
                |_| {
                    human(&format_args!("no known crate named `{}`", &*dep.name))
                },
            )?;
            if dep.version_req == semver::VersionReq::parse("*").unwrap() {
                return Err(human(
                    "wildcard (`*`) dependency constraints are not allowed \
                     on crates.io. See http://doc.crates.io/faq.html#can-\
                     libraries-use--as-a-version-for-their-dependencies for more \
                     information",
                ));
            }
            let features: Vec<_> = dep.features.iter().map(|s| &**s).collect();

            Ok((
                git::Dependency {
                    name: dep.name.to_string(),
                    req: dep.version_req.to_string(),
                    features: features.iter().map(|s| s.to_string()).collect(),
                    optional: dep.optional,
                    default_features: dep.default_features,
                    target: dep.target.clone(),
                    kind: dep.kind.or(Some(Kind::Normal)),
                },
                NewDependency {
                    version_id: version_id,
                    crate_id: krate.id,
                    req: dep.version_req.to_string(),
                    kind: dep.kind.unwrap_or(Kind::Normal) as i32,
                    optional: dep.optional,
                    default_features: dep.default_features,
                    features: features,
                    target: dep.target.as_ref().map(|s| &**s),
                },
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let (git_deps, new_dependencies): (Vec<_>, Vec<_>) =
        git_and_new_dependencies.into_iter().unzip();

    insert(&new_dependencies)
        .into(dependencies::table)
        .execute(conn)?;

    Ok(git_deps)
}

impl Queryable<dependencies::SqlType, Pg> for Dependency {
    type Row = (i32, i32, i32, String, bool, bool, Vec<String>, Option<String>, i32);

    fn build(row: Self::Row) -> Self {
        Dependency {
            id: row.0,
            version_id: row.1,
            crate_id: row.2,
            req: semver::VersionReq::parse(&row.3).unwrap(),
            optional: row.4,
            default_features: row.5,
            features: row.6,
            target: row.7,
            kind: match row.8 {
                0 => Kind::Normal,
                1 => Kind::Build,
                2 => Kind::Dev,
                n => panic!("unknown kind: {}", n),
            },
        }
    }
}

// FIXME: We can derive this in the next release of Diesel
impl Queryable<(dependencies::SqlType, Integer, Text), Pg> for ReverseDependency {
    type Row = (<Dependency as Queryable<dependencies::SqlType, Pg>>::Row, i32, String);

    fn build((dep_row, downloads, name): Self::Row) -> Self {
        ReverseDependency {
            dependency: Dependency::build(dep_row),
            crate_downloads: downloads,
            crate_name: name,
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
            },
        }
    }

    fn table_name(_: Option<Dependency>) -> &'static str {
        "dependencies"
    }
}
