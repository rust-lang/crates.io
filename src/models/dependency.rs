use diesel::sql_types::{BigInt, Text};

use crate::models::{Crate, Version};
use crate::schema::*;
use crates_io_diesel_helpers::pg_enum;
use crates_io_index::DependencyKind as IndexDependencyKind;

#[derive(Identifiable, Associations, Debug, Queryable, QueryableByName, Selectable)]
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
    #[diesel(sql_type = BigInt)]
    pub crate_downloads: i64,
    #[diesel(sql_type = Text, column_name = crate_name)]
    pub name: String,
}

pg_enum! {
    pub enum DependencyKind {
        Normal = 0,
        Build = 1,
        Dev = 2,
    }
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
