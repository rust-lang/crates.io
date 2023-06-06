use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::sql_types::{Integer, Text};

use crate::models::{Crate, Version};
use crate::schema::*;
use crates_io_index::DependencyKind as IndexDependencyKind;

#[derive(Identifiable, Associations, Debug, Queryable, QueryableByName)]
#[diesel(
    table_name = dependencies,
    check_for_backend(diesel::pg::Pg),
    belongs_to(Version),
    belongs_to(Crate),
)]
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
    pub explicit_name: Option<String>,
}

#[derive(Debug, QueryableByName)]
pub struct ReverseDependency {
    #[diesel(embed)]
    pub dependency: Dependency,
    #[diesel(sql_type = Integer)]
    pub crate_downloads: i32,
    #[diesel(sql_type = Text, column_name = crate_name)]
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

impl From<IndexDependencyKind> for DependencyKind {
    fn from(dk: IndexDependencyKind) -> Self {
        match dk {
            IndexDependencyKind::Normal => DependencyKind::Normal,
            IndexDependencyKind::Build => DependencyKind::Build,
            IndexDependencyKind::Dev => DependencyKind::Dev,
        }
    }
}

impl From<DependencyKind> for IndexDependencyKind {
    fn from(dk: DependencyKind) -> Self {
        match dk {
            DependencyKind::Normal => IndexDependencyKind::Normal,
            DependencyKind::Build => IndexDependencyKind::Build,
            DependencyKind::Dev => IndexDependencyKind::Dev,
        }
    }
}

impl FromSql<Integer, Pg> for DependencyKind {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> deserialize::Result<Self> {
        match <i32 as FromSql<Integer, Pg>>::from_sql(bytes)? {
            0 => Ok(DependencyKind::Normal),
            1 => Ok(DependencyKind::Build),
            2 => Ok(DependencyKind::Dev),
            n => Err(format!("unknown kind: {n}").into()),
        }
    }
}
