use crate::schema::trustpub_configs_github;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

#[derive(Debug, Identifiable, Queryable, Selectable)]
#[diesel(table_name = trustpub_configs_github, check_for_backend(diesel::pg::Pg))]
pub struct GitHubConfig {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub crate_id: i32,
    pub repository_owner: String,
    pub repository_owner_id: i32,
    pub repository_name: String,
    pub workflow_filename: String,
    pub environment: Option<String>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = trustpub_configs_github, check_for_backend(diesel::pg::Pg))]
pub struct NewGitHubConfig<'a> {
    pub crate_id: i32,
    pub repository_owner: &'a str,
    pub repository_owner_id: i32,
    pub repository_name: &'a str,
    pub workflow_filename: &'a str,
    pub environment: Option<&'a str>,
}

impl NewGitHubConfig<'_> {
    pub async fn insert(&self, conn: &mut AsyncPgConnection) -> QueryResult<GitHubConfig> {
        self.insert_into(trustpub_configs_github::table)
            .returning(GitHubConfig::as_returning())
            .get_result(conn)
            .await
    }
}
