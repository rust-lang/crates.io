use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::Integer;

use crate::git;
use crate::util::errors::{cargo_err, AppResult};

use crate::models::{Crate, Version};
use crate::schema::*;
use crate::views::EncodableCrateDependency;

pub const WILDCARD_ERROR_MESSAGE: &str = "wildcard (`*`) dependency constraints are not allowed \
     on crates.io. See https://doc.rust-lang.org/cargo/faq.html#can-\
     libraries-use--as-a-version-for-their-dependencies for more \
     information";

#[derive(Identifiable, Associations, Debug, Queryable, QueryableByName)]
#[belongs_to(Version)]
#[belongs_to(Crate)]
#[table_name = "dependencies"]
pub struct Dependency {
    pub id: i32,
    pub version_id: i32,
    pub crate_id: i32,
    pub req: String,
    pub optional: bool,
    pub default_features: bool,
    pub features: Vec<String>,
    pub target: Option<String>,
    pub kind: DependencyKind,
}

#[derive(Debug, QueryableByName)]
pub struct ReverseDependency {
    #[diesel(embed)]
    pub dependency: Dependency,
    #[sql_type = "::diesel::sql_types::Integer"]
    pub crate_downloads: i32,
    #[sql_type = "::diesel::sql_types::Text"]
    #[column_name = "crate_name"]
    pub name: String,
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug, FromSqlRow)]
#[serde(rename_all = "lowercase")]
#[repr(u32)]
pub enum DependencyKind {
    Normal = 0,
    Build = 1,
    Dev = 2,
    // if you add a kind here, be sure to update `from_row` below.
}

pub fn add_dependencies(
    conn: &PgConnection,
    deps: &[EncodableCrateDependency],
    target_version_id: i32,
) -> AppResult<Vec<git::Dependency>> {
    use self::dependencies::dsl::*;
    use diesel::insert_into;

    let git_and_new_dependencies = deps
        .iter()
        .map(|dep| {
            if let Some(registry) = &dep.registry {
                if !registry.is_empty() {
                    return Err(cargo_err(&format_args!("Dependency `{}` is hosted on another registry. Cross-registry dependencies are not permitted on crates.io.", &*dep.name)));
                }
            }

            // Match only identical names to ensure the index always references the original crate name
            let krate:Crate = Crate::by_exact_name(&dep.name)
                .first(&*conn)
                .map_err(|_| cargo_err(&format_args!("no known crate named `{}`", &*dep.name)))?;
            if semver::VersionReq::parse(&dep.version_req.0) == semver::VersionReq::parse("*") {
                return Err(cargo_err(WILDCARD_ERROR_MESSAGE));
            }

            // If this dependency has an explicit name in `Cargo.toml` that
            // means that the `name` we have listed is actually the package name
            // that we're depending on. The `name` listed in the index is the
            // Cargo.toml-written-name which is what cargo uses for
            // `--extern foo=...`
            let (name, package) = match &dep.explicit_name_in_toml {
                Some(explicit) => (explicit.to_string(), Some(dep.name.to_string())),
                None => (dep.name.to_string(), None),
            };

            Ok((
                git::Dependency {
                    name,
                    req: dep.version_req.to_string(),
                    features: dep.features.iter().map(|s| s.0.to_string()).collect(),
                    optional: dep.optional,
                    default_features: dep.default_features,
                    target: dep.target.clone(),
                    kind: dep.kind.or(Some(DependencyKind::Normal)),
                    package,
                },
                (
                    version_id.eq(target_version_id),
                    crate_id.eq(krate.id),
                    req.eq(dep.version_req.to_string()),
                    dep.kind.map(|k| kind.eq(k as i32)),
                    optional.eq(dep.optional),
                    default_features.eq(dep.default_features),
                    features.eq(&dep.features),
                    target.eq(dep.target.as_deref()),
                ),
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let (git_deps, new_dependencies): (Vec<_>, Vec<_>) =
        git_and_new_dependencies.into_iter().unzip();

    insert_into(dependencies)
        .values(&new_dependencies)
        .execute(conn)?;

    Ok(git_deps)
}

impl FromSql<Integer, Pg> for DependencyKind {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        match <i32 as FromSql<Integer, Pg>>::from_sql(bytes)? {
            0 => Ok(DependencyKind::Normal),
            1 => Ok(DependencyKind::Build),
            2 => Ok(DependencyKind::Dev),
            n => Err(format!("unknown kind: {}", n).into()),
        }
    }
}
