use crate::worker::Environment;
use crates_io_worker::BackgroundJob;
use diesel::sql_query;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};
use tracing::info;

pub const SQL_DIRECTORY: &str = "sql";

/// Run the sql contained in the file specified by the given name that exists in the `sql`
/// directory. That is, it's expected that the files be reviewed and checked into the repo before
/// being run in production with this job.
///
/// The SQL is expected to run in a short amount of time so that this job doesn't overload the
/// production database. Using this job rather than a schema migration means deploys can happen
/// whenever and this SQL can be run independently at a later time of lower traffic.
#[derive(Serialize, Deserialize)]
pub struct RunSql {
    file_name: PathBuf,
}

impl RunSql {
    pub fn new(file_name: impl AsRef<Path>) -> Self {
        Self {
            file_name: file_name.as_ref().into(),
        }
    }
}

impl BackgroundJob for RunSql {
    const JOB_NAME: &'static str = "run_sql";
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        let mut conn = env.deadpool.get().await?;
        Ok(run_sql(&mut conn, &self.file_name).await?)
    }
}

/// Errors that may happen while running SQL.
#[derive(Debug, thiserror::Error)]
pub enum RunSqlError {
    FileDoesNotExist {
        file_name: String,
    },

    UnhandledIoError {
        error: std::io::Error,
        file_name: String,
    },

    Sql {
        error: diesel::result::Error,
    },
}

impl std::fmt::Display for RunSqlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::FileDoesNotExist { file_name } => {
                write!(
                    f,
                    "SQL file named `{file_name}` does not exist in the `{SQL_DIRECTORY}` directory"
                )
            }
            Self::UnhandledIoError { error, file_name } => {
                write!(
                    f,
                    "I/O error while reading SQL file named {file_name}: {error}"
                )
            }
            Self::Sql { error } => {
                write!(f, "SQL error while executing SQL: {error}")
            }
        }
    }
}

async fn run_sql(
    conn: &mut AsyncPgConnection,
    file_name: impl AsRef<Path>,
) -> Result<(), RunSqlError> {
    let file_name_display = file_name.as_ref().display().to_string();
    let sql_file = Path::new(SQL_DIRECTORY).join(file_name.as_ref());
    let sql = std::fs::read_to_string(sql_file).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => RunSqlError::FileDoesNotExist {
            file_name: file_name_display.clone(),
        },
        _ => RunSqlError::UnhandledIoError {
            error,
            file_name: file_name_display.clone(),
        },
    })?;

    let start = Instant::now();

    sql_query(sql)
        .execute(conn)
        .await
        .map_err(|error| RunSqlError::Sql { error })?;

    info!(
        elapsed_ms = start.elapsed().as_millis(),
        file_name_display, "run_sql job completed"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crates_io_test_db::TestDatabase;

    #[tokio::test]
    async fn nonexistent_sql_file_errors() {
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        assert_eq!(
            run_sql(&mut conn, "definitely_does_not_exist_do_not_create_this")
                .await
                .unwrap_err()
                .to_string(),
            "SQL file named `definitely_does_not_exist_do_not_create_this` \
            does not exist in the `sql` directory",
        );
    }

    #[tokio::test]
    async fn file_exists_but_not_in_sql_dir_errors() {
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        let temp_sql_file = tempfile::Builder::new().tempfile().unwrap();
        let sql = "SELECT 0 as num_remaining;";
        std::fs::write(&temp_sql_file, sql).unwrap();

        let file_name = temp_sql_file.as_ref().file_name().unwrap();
        assert_eq!(
            run_sql(&mut conn, &file_name)
                .await
                .unwrap_err()
                .to_string(),
            format!(
                "SQL file named `{}` does not exist in the `sql` directory",
                file_name.to_string_lossy()
            ),
        );
    }

    #[tokio::test]
    async fn error_if_sql_errors() {
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        let temp_sql_file = tempfile::Builder::new().tempfile_in("sql").unwrap();
        let sql = "this is invalid sql";
        std::fs::write(&temp_sql_file, sql).unwrap();

        let err = run_sql(&mut conn, temp_sql_file.as_ref().file_name().unwrap())
            .await
            .unwrap_err()
            .to_string();
        assert_eq!(
            "SQL error while executing SQL: syntax error at or near \"this\"",
            err
        );
    }

    #[tokio::test]
    async fn ok_if_sql_succeeds() {
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        let temp_sql_file = tempfile::Builder::new().tempfile_in("sql").unwrap();
        let sql = "SELECT 0 as num_remaining;";
        std::fs::write(&temp_sql_file, sql).unwrap();

        run_sql(&mut conn, temp_sql_file.as_ref().file_name().unwrap())
            .await
            .unwrap();
    }
}
