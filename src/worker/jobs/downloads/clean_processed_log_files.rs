use crate::schema::processed_log_files;
use crate::worker::Environment;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use std::sync::Arc;

/// This job is responsible for cleaning up old entries in the
/// `processed_log_files` table.
///
/// Rows older than one week will be deleted.
#[derive(Serialize, Deserialize)]
pub struct CleanProcessedLogFiles;

impl BackgroundJob for CleanProcessedLogFiles {
    const JOB_NAME: &'static str = "clean_processed_log_files";
    const DEDUPLICATED: bool = true;
    const QUEUE: &'static str = "downloads";

    type Context = Arc<Environment>;

    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        let mut conn = env.deadpool.get().await?;
        Ok(run(&mut conn).await?)
    }
}

async fn run(conn: &mut AsyncPgConnection) -> QueryResult<()> {
    let filter = processed_log_files::time.lt(cut_off_date());

    diesel::delete(processed_log_files::table.filter(filter))
        .execute(conn)
        .await?;

    Ok(())
}

fn cut_off_date() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now() - chrono::TimeDelta::try_weeks(1).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use crates_io_test_db::TestDatabase;
    use diesel_async::{AsyncConnection, AsyncPgConnection};
    use insta::assert_debug_snapshot;

    #[tokio::test]
    async fn test_cleanup() {
        let test_db = TestDatabase::new();
        let mut conn = AsyncPgConnection::establish(test_db.url()).await.unwrap();

        let now = chrono::Utc::now();
        let cut_off_date = cut_off_date();
        let one_hour = chrono::Duration::try_hours(1).unwrap();

        let inserts = vec![
            ("very-old-file", cut_off_date - one_hour * 30 * 24),
            ("old-file", cut_off_date - one_hour),
            ("newish-file", cut_off_date + one_hour),
            ("brand-new-file", now),
            ("future-file", now + one_hour * 7 * 24),
        ];
        insert(&mut conn, inserts).await;
        assert_debug_snapshot!(paths_in_table(&mut conn).await, @r#"
        [
            "very-old-file",
            "old-file",
            "newish-file",
            "brand-new-file",
            "future-file",
        ]
        "#);

        run(&mut conn).await.unwrap();
        assert_debug_snapshot!(paths_in_table(&mut conn).await, @r#"
        [
            "newish-file",
            "brand-new-file",
            "future-file",
        ]
        "#);
    }

    /// Insert a list of paths and times into the `processed_log_files` table.
    async fn insert(conn: &mut AsyncPgConnection, inserts: Vec<(&str, DateTime<Utc>)>) {
        let inserts = inserts
            .into_iter()
            .map(|(path, time)| {
                (
                    processed_log_files::path.eq(path),
                    processed_log_files::time.eq(time),
                )
            })
            .collect::<Vec<_>>();

        diesel::insert_into(processed_log_files::table)
            .values(&inserts)
            .execute(conn)
            .await
            .unwrap();
    }

    /// Read all paths from the `processed_log_files` table.
    async fn paths_in_table(conn: &mut AsyncPgConnection) -> Vec<String> {
        processed_log_files::table
            .select(processed_log_files::path)
            .load::<String>(conn)
            .await
            .unwrap()
    }
}
