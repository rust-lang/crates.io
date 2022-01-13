use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::sql_types::Integer;

use crate::models::{Crate, Version};
use crate::schema::*;

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

impl FromSql<Integer, Pg> for DependencyKind {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        match <i32 as FromSql<Integer, Pg>>::from_sql(bytes)? {
            0 => Ok(DependencyKind::Normal),
            1 => Ok(DependencyKind::Build),
            2 => Ok(DependencyKind::Dev),
            n => Err(format!("unknown kind: {n}").into()),
        }
    }
}
