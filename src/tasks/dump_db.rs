use std::{
    collections::{BTreeMap, VecDeque},
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

/// Filtering information for a single table. The `dependencies` field is only
/// used to determine the order of the tables in the generated import script,
/// and should list all tables the current tables refers to with foreign key
/// constraints on public columns. The `filter` field is a valid SQL expression
/// used in a `WHERE` clause to filter the rows of the table. The `columns`
/// field maps column names to their respective visibilities.
#[derive(Clone, Debug, Deserialize)]
struct TableConfig {
    #[serde(default)]
    dependencies: Vec<String>,
    filter: Option<String>,
    columns: BTreeMap<String, ColumnVisibility>,
}

/// Subset of the configuration data to be passed on to the Handlbars template.
#[derive(Debug, Serialize)]
struct HandlebarsTableContext<'a> {
    name: &'a str,
    filter: Option<&'a str>,
    columns: String,
}

impl TableConfig {
    fn handlebars_context<'a>(&'a self, name: &'a str) -> Option<HandlebarsTableContext<'a>> {
        let columns = self
            .columns
            .iter()
            .filter(|&(_, &vis)| vis == ColumnVisibility::Public)
            .map(|(col, _)| format!("\"{}\"", col))
            .collect::<Vec<String>>()
            .join(", ");
        if columns.is_empty() {
            None
        } else {
            Some(HandlebarsTableContext {
                name,
                filter: self.filter.as_ref().map(String::as_str),
                columns,
            })
        }
    }
}

/// Maps table names to the respective configurations. Used to load `dump_db.toml`.
#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
struct VisibilityConfig(BTreeMap<String, TableConfig>);

/// Subset of the configuration data to be passed on to the Handlbars template.
#[derive(Debug, Serialize)]
struct HandlebarsContext<'a> {
    tables: Vec<HandlebarsTableContext<'a>>,
}

impl VisibilityConfig {
    /// Sort the tables in a way that dependencies come before dependent tables.
    ///
    /// Returns a vector of table names.
    fn topological_sort(&self) -> Vec<&str> {
        let mut result = Vec::new();
        let mut num_deps = BTreeMap::new();
        let mut rev_deps: BTreeMap<&str, Vec<&str>> = self
            .0
            .iter()
            .map(|(table, _)| (table.as_str(), vec![]))
            .collect();
        for (table, config) in self.0.iter() {
            num_deps.insert(table.as_str(), config.dependencies.len());
            for dep in &config.dependencies {
                rev_deps.get_mut(dep.as_str()).unwrap().push(table.as_str());
            }
        }
        let mut ready: VecDeque<&str> = num_deps
            .iter()
            .filter(|(_, &count)| count == 0)
            .map(|(&table, _)| table)
            .collect();
        while let Some(table) = ready.pop_front() {
            result.push(table);
            for dep in &rev_deps[table] {
                *num_deps.get_mut(dep).unwrap() -= 1;
                if num_deps[dep] == 0 {
                    ready.push_back(dep);
                }
            }
        }
        assert_eq!(
            self.0.len(),
            result.len(),
            "circular dependencies in DB dump config detected",
        );
        result
    }

    fn handlebars_context(&self) -> HandlebarsContext<'_> {
        let tables = self
            .topological_sort()
            .into_iter()
            .filter_map(|table| self.0[table].handlebars_context(table))
            .collect();
        HandlebarsContext { tables }
    }

    fn render_template(&self, template: &str) -> String {
        let context = self.handlebars_context();
        let mut handlebars = handlebars::Handlebars::new();
        handlebars.register_escape_fn(handlebars::no_escape);
        handlebars.render_template(template, &context).unwrap()
    }

    fn gen_export_script(&self) -> String {
        self.render_template(include_str!("dump-export.hbs"))
    }

    #[allow(dead_code)]
    fn gen_import_script(&self) -> String {
        self.render_template(include_str!("dump-import.hbs"))
    }
}

fn run_psql(
    config: &VisibilityConfig,
    database_url: &str,
    export_dir: &Path,
) -> Result<(), PerformError> {
    use std::io::prelude::*;
    use std::process::{Command, Stdio};

    let psql_script = config.gen_export_script();
    let mut psql = Command::new("psql")
        .arg(database_url)
        .current_dir(export_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdin = psql.stdin.take().unwrap();
    let input_thread = std::thread::spawn(move || -> std::io::Result<()> {
        stdin.write_all(psql_script.as_bytes())?;
        Ok(())
    });
    let output = psql.wait_with_output()?;
    input_thread.join().unwrap()?;
    if !output.stderr.is_empty() {
        Err(format!(
            "Error while executing psql: {}",
            String::from_utf8_lossy(&output.stderr)
        ))?;
    }
    if !output.status.success() {
        Err("psql did not finish successfully.")?;
    }
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
