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

pg_enum! {
    pub enum DependencyKind {
        Normal = 0,
        Build = 1,
        Dev = 2,
    }
}
