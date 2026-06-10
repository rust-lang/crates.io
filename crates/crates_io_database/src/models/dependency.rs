use crate::models::helpers::with_count::*;
use crate::models::{Crate, Version};
use crate::pg_enum;
use crate::schema::*;
use crates_io_index::DependencyKind as IndexDependencyKind;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Integer, Text};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use tracing::instrument;

#[derive(Identifiable, Associations, Debug, HasQuery, QueryableByName)]
#[diesel(
    table_name = dependencies,
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

impl ReverseDependency {
    #[instrument(skip_all, fields(crate_id))]
    pub async fn for_crate(
        crate_id: i32,
        mut conn: &AsyncPgConnection,
        offset: i64,
        limit: i64,
    ) -> QueryResult<(Vec<Self>, i64)> {
        let rows: Vec<WithCount<Self>> =
            diesel::sql_query(include_str!("krate_reverse_dependencies.sql"))
                .bind::<Integer, _>(crate_id)
                .bind::<BigInt, _>(offset)
                .bind::<BigInt, _>(limit)
                .load(&mut conn)
                .await?;

        Ok(rows.records_and_total())
    }
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
