use crate::schema::trustpub_configs_gitlab;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::Serialize;

#[derive(Debug, Identifiable, Queryable, Selectable, Serialize)]
#[diesel(table_name = trustpub_configs_gitlab, check_for_backend(diesel::pg::Pg))]
pub struct GitLabConfig {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub crate_id: i32,
    pub namespace: String,
    pub project: String,
    pub workflow_filepath: String,
    pub environment: Option<String>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = trustpub_configs_gitlab, check_for_backend(diesel::pg::Pg))]
pub struct NewGitLabConfig<'a> {
    pub crate_id: i32,
    pub namespace: &'a str,
    pub project: &'a str,
    pub workflow_filepath: &'a str,
    pub environment: Option<&'a str>,
}

impl NewGitLabConfig<'_> {
    pub async fn insert(&self, conn: &mut AsyncPgConnection) -> QueryResult<GitLabConfig> {
        self.insert_into(trustpub_configs_gitlab::table)
            .returning(GitLabConfig::as_returning())
            .get_result(conn)
            .await
    }
}
