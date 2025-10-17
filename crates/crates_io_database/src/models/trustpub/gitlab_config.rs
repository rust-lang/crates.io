use crate::schema::trustpub_configs_gitlab;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::Serialize;

#[derive(Debug, Identifiable, HasQuery, Serialize)]
#[diesel(table_name = trustpub_configs_gitlab)]
pub struct GitLabConfig {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub crate_id: i32,
    pub namespace: String,
    pub namespace_id: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::krate::*;
    use crate::schema::crates;
    use crates_io_test_db::TestDatabase;
    use diesel_async::RunQueryDsl;
    use insta::assert_debug_snapshot;

    #[tokio::test]
    async fn test_gitlab_config_insert_and_retrieve() {
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        // Create a test crate first
        let test_crate = diesel::insert_into(crates::table)
            .values((crates::name.eq("test-crate"),))
            .returning(Crate::as_returning())
            .get_result(&mut conn)
            .await
            .unwrap();

        // Create a new GitLab config
        let new_config = NewGitLabConfig {
            crate_id: test_crate.id,
            namespace: "rust-lang",
            project: "cargo",
            workflow_filepath: ".gitlab-ci.yml",
            environment: Some("production"),
        };

        // Insert the config
        let inserted_config = new_config.insert(&mut conn).await.unwrap();

        // Retrieve the config
        let retrieved_config = GitLabConfig::query()
            .filter(trustpub_configs_gitlab::id.eq(inserted_config.id))
            .first(&mut conn)
            .await
            .unwrap();

        // Snapshot test the structure
        insta::with_settings!({ filters => vec![(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z", "[datetime]")] }, {
            assert_debug_snapshot!(retrieved_config, @r#"
            GitLabConfig {
                id: 1,
                created_at: [datetime],
                crate_id: 1,
                namespace: "rust-lang",
                namespace_id: None,
                project: "cargo",
                workflow_filepath: ".gitlab-ci.yml",
                environment: Some(
                    "production",
                ),
            }
            "#);
        });
    }
}
