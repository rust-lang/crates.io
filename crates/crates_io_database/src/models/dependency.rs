use crate::models::{Crate, Version};
use crate::pg_enum;
use crate::schema::*;
use crates_io_index::DependencyKind as IndexDependencyKind;
use diesel::prelude::*;
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

#[derive(Debug, HasQuery)]
#[diesel(
    base_query = reverse_dependencies::table
        .inner_join(crates::table.on(crates::id.eq(reverse_dependencies::dependent_crate_id)))
        .inner_join(dependencies::table.on(dependencies::id.eq(reverse_dependencies::dependency_id)))
)]
pub struct ReverseDependency {
    #[diesel(embed)]
    pub dependency: Dependency,
    #[diesel(select_expression = reverse_dependencies::dependent_downloads)]
    pub crate_downloads: i64,
    #[diesel(select_expression = crates::name)]
    pub name: String,
}

impl ReverseDependency {
    #[instrument(skip_all, fields(crate_id))]
    pub async fn for_crate(
        crate_id: i32,
        conn: &AsyncPgConnection,
        offset: i64,
        limit: i64,
    ) -> QueryResult<(Vec<Self>, i64)> {
        let records = Self::page_for_crate(crate_id, conn, offset, limit).await?;
        let total = Self::count_for_crate(crate_id, conn).await?;
        Ok((records, total))
    }

    /// Returns a page of reverse dependencies, ordered by the dependent crate's
    /// total downloads.
    async fn page_for_crate(
        crate_id: i32,
        mut conn: &AsyncPgConnection,
        offset: i64,
        limit: i64,
    ) -> QueryResult<Vec<Self>> {
        Self::query()
            .filter(reverse_dependencies::target_crate_id.eq(crate_id))
            .order((
                reverse_dependencies::dependent_downloads.desc(),
                reverse_dependencies::dependent_crate_id.desc(),
            ))
            .offset(offset)
            .limit(limit)
            .load(&mut conn)
            .await
    }

    /// Returns the total number of reverse dependencies for the crate.
    async fn count_for_crate(crate_id: i32, mut conn: &AsyncPgConnection) -> QueryResult<i64> {
        reverse_dependencies::table
            .filter(reverse_dependencies::target_crate_id.eq(crate_id))
            .count()
            .get_result(&mut conn)
            .await
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
