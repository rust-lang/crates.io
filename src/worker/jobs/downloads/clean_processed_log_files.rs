use crate::schema::processed_log_files;
use crate::util::diesel::Conn;
use crate::worker::Environment;
use anyhow::anyhow;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use std::sync::Arc;

/// This job is responsible for cleaning up old entries in the
/// `processed_log_files` table.
///
/// Rows older than one week will be deleted.
#[derive(Serialize, Deserialize)]
pub struct CleanProcessedLogFiles;

impl BackgroundJob for CleanProcessedLogFiles {
    const JOB_NAME: &'static str = "clean_processed_log_files";
    const QUEUE: &'static str = "downloads";

    type Context = Arc<Environment>;

    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        let conn = env.deadpool.get().await?;
        conn.interact(run)
            .await
            .map_err(|err| anyhow!(err.to_string()))??;

        Ok(())
    }
}

fn run(conn: &mut impl Conn) -> QueryResult<()> {
    let filter = processed_log_files::time.lt(cut_off_date());
    diesel::delete(processed_log_files::table.filter(filter)).execute(conn)?;

    Ok(())
}

fn cut_off_date() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now() - chrono::TimeDelta::try_weeks(1).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_db_connection;
    use chrono::{DateTime, Utc};
    use insta::assert_debug_snapshot;

    #[test]
    fn test_cleanup() {
        let (_test_db, conn) = &mut test_db_connection();

        let now = chrono::Utc::now();
        let cut_off_date = cut_off_date();
        let one_hour = chrono::Duration::try_hours(1).unwrap();

        insert(
            conn,
            vec![
                ("very-old-file", cut_off_date - one_hour * 30 * 24),
                ("old-file", cut_off_date - one_hour),
                ("newish-file", cut_off_date + one_hour),
                ("brand-new-file", now),
                ("future-file", now + one_hour * 7 * 24),
            ],
        );
        assert_debug_snapshot!(paths_in_table(conn), @r###"
        [
            "very-old-file",
            "old-file",
            "newish-file",
            "brand-new-file",
            "future-file",
        ]
        "###);

        run(conn).unwrap();
        assert_debug_snapshot!(paths_in_table(conn), @r###"
        [
            "newish-file",
            "brand-new-file",
            "future-file",
        ]
        "###);
    }

    /// Insert a list of paths and times into the `processed_log_files` table.
    fn insert(conn: &mut PgConnection, inserts: Vec<(&str, DateTime<Utc>)>) {
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
            .unwrap();
    }

    /// Read all paths from the `processed_log_files` table.
    fn paths_in_table(conn: &mut PgConnection) -> Vec<String> {
        processed_log_files::table
            .select(processed_log_files::path)
            .load::<String>(conn)
            .unwrap()
    }
}
