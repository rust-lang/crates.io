use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use crate::{background_jobs::Environment, uploaders::Uploader, util::errors::std_error_no_send};

use scopeguard::defer;
use swirl::PerformError;

/// Create CSV dumps of the public information in the database, wrap them in a
/// tarball and upload to S3.
#[swirl::background_job]
pub fn dump_db(
    env: &Environment,
    database_url: String,
    target_name: String,
) -> Result<(), PerformError> {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d-%H%M%S").to_string();
    let export_dir = std::env::temp_dir().join("dump-db").join(timestamp);
    std::fs::create_dir_all(&export_dir)?;
    defer! {{
        std::fs::remove_dir_all(&export_dir).unwrap();
    }}
    let visibility_config = toml::from_str(include_str!("dump-db.toml")).unwrap();
    run_psql(&visibility_config, &database_url, &export_dir)?;
    let tarball = create_tarball(&export_dir)?;
    defer! {{
        std::fs::remove_file(&tarball).unwrap();
    }}
    upload_tarball(&tarball, &target_name, &env.uploader)?;
    println!("Database dump uploaded to {}.", &target_name);
    Ok(())
}

/// An enum indicating whether a column is included in the database dumps.
/// Public columns are included, private are not.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum ColumnVisibility {
    Private,
    Public,
}

/// Filtering information for a single table. The `filter` field is a valid SQL
/// expression used in a `WHERE` clause to filter the rows of the table. The
/// `columns` field maps column names to their respective visibilities.
#[derive(Clone, Debug, Deserialize)]
struct TableConfig {
    filter: Option<String>,
    columns: BTreeMap<String, ColumnVisibility>,
}

impl TableConfig {
    fn columns_str(&self) -> String {
        self.columns
            .iter()
            .filter(|&(_, &vis)| vis == ColumnVisibility::Public)
            .map(|(col, _)| format!("\"{}\"", col))
            .collect::<Vec<String>>()
            .join(", ")
    }

    fn view_sql(&self, table: &str) -> String {
        self.filter
            .as_ref()
            .map(|filter| {
                format!(
                    r#"
                    CREATE TEMPORARY VIEW "dump_db_{table}" AS (
                        SELECT {columns}
                        FROM "{table}"
                        WHERE {filter}
                    );
                    "#,
                    table = table,
                    columns = self.columns_str(),
                    filter = filter,
                )
            })
            .unwrap_or_default()
    }

    fn copy_sql(&self, table: &str) -> String {
        if self.filter.is_some() {
            format!(
                r#"
                \copy (SELECT * FROM "dump_db_{table}") TO '{table}.csv' WITH CSV HEADER
                "#,
                table = table,
            )
        } else {
            let cols_str = self.columns_str();
            if cols_str.is_empty() {
                String::new()
            } else {
                format!(
                    r#"
                    \copy "{table}" ({columns}) TO '{table}.csv' WITH CSV HEADER
                    "#,
                    table = table,
                    columns = cols_str,
                )
            }
        }
    }
}

/// Maps table names to the respective configurations. Used to load `dump_db.toml`.
#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
struct VisibilityConfig(BTreeMap<String, TableConfig>);

impl VisibilityConfig {
    fn gen_psql_script(&self) -> String {
        let view_sql = self
            .0
            .iter()
            .map(|(table, config)| config.view_sql(table))
            .collect::<Vec<String>>()
            .concat();
        let copy_sql = self
            .0
            .iter()
            .map(|(table, config)| config.copy_sql(table))
            .collect::<Vec<String>>()
            .concat();
        format!(
            r#"
            BEGIN;
            {view_sql}
            COMMIT;
            BEGIN ISOLATION LEVEL SERIALIZABLE READ ONLY DEFERRABLE;
            {copy_sql}
            COMMIT;
            "#,
            view_sql = view_sql,
            copy_sql = copy_sql,
        )
    }
}

fn run_psql(
    config: &VisibilityConfig,
    database_url: &str,
    export_dir: &Path,
) -> Result<(), PerformError> {
    use std::io::prelude::*;
    use std::process::{Command, Stdio};

    let psql_script = config.gen_psql_script();
    // TODO Redirect stdout and stderr to avoid polluting the worker logs.
    let mut psql = Command::new("psql")
        .arg(database_url)
        .current_dir(export_dir)
        .stdin(Stdio::piped())
        .spawn()?;
    let mut stdin = psql.stdin.take().unwrap();
    stdin.write_all(psql_script.as_bytes())?;
    drop(stdin);
    psql.wait()?;
    Ok(())
}

fn create_tarball(export_dir: &Path) -> Result<PathBuf, PerformError> {
    let tarball_name = export_dir.with_extension("tar.gz");
    let tarball = std::fs::File::create(&tarball_name)?;
    let encoder = flate2::write::GzEncoder::new(tarball, flate2::Compression::default());
    let mut archive = tar::Builder::new(encoder);
    archive.append_dir_all(export_dir.file_name().unwrap(), &export_dir)?;
    Ok(tarball_name)
}

fn upload_tarball(
    tarball: &Path,
    target_name: &str,
    uploader: &Uploader,
) -> Result<(), PerformError> {
    let client = reqwest::Client::new();
    let tarfile = std::fs::File::open(tarball)?;
    let content_length = tarfile.metadata()?.len();
    // TODO Figure out the correct content type.
    uploader
        .upload(
            &client,
            target_name,
            tarfile,
            content_length,
            "application/gzip",
        )
        .map_err(std_error_no_send)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::pg_connection;
    use diesel::prelude::*;
    use std::collections::HashSet;

    /// Test whether the schema in the visibility configuration matches the test database.
    #[test]
    fn check_visibility_config() {
        let conn = pg_connection();
        let db_columns: HashSet<_> = get_db_columns(&conn)
            .into_iter()
            .map(|c| (c.table_name, c.column_name))
            .collect();
        let visibility_config: VisibilityConfig =
            toml::from_str(include_str!("dump-db.toml")).unwrap();
        let vis_columns: HashSet<_> = visibility_config
            .0
            .iter()
            .flat_map(|(table, config)| {
                config
                    .columns
                    .iter()
                    .map(move |(column, _)| (table.clone(), column.clone()))
            })
            .collect();
        let mut errors = vec![];
        for (table, col) in db_columns.difference(&vis_columns) {
            errors.push(format!(
                "No visibility information for columns {}.{}.",
                table, col
            ));
        }
        for (table, col) in vis_columns.difference(&db_columns) {
            errors.push(format!(
                "Column {}.{} does not exist in the database.",
                table, col
            ));
        }
        assert!(
            errors.is_empty(),
            "The visibility configuration does not match the database schema:\n{}",
            errors.join("\n  - "),
        );
    }

    mod information_schema {
        table! {
            information_schema.columns (table_schema, table_name, column_name) {
                table_schema -> Text,
                table_name -> Text,
                column_name -> Text,
                ordinal_position -> Integer,
            }
        }
    }

    #[derive(Debug, PartialEq, Queryable)]
    struct Column {
        table_name: String,
        column_name: String,
    }

    fn get_db_columns(conn: &PgConnection) -> Vec<Column> {
        use information_schema::columns::dsl::*;
        columns
            .select((table_name, column_name))
            .filter(table_schema.eq("public"))
            .order_by((table_name, ordinal_position))
            .load(conn)
            .unwrap()
    }
}
