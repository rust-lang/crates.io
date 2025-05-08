use crate::configuration::{ColumnVisibility, SequenceConfig, TableConfig, VisibilityConfig};
use anyhow::Context;
use serde::Serialize;
use std::{fs::File, path::Path};
use tracing::debug;

pub fn gen_scripts(export_script: &Path, import_script: &Path) -> anyhow::Result<()> {
    let config = VisibilityConfig::get();
    let export_sql = File::create(export_script).context("Failed to create export script file")?;
    let import_sql = File::create(import_script).context("Failed to create import script file")?;
    config.gen_psql_scripts(export_sql, import_sql)
}

/// Subset of the configuration data to be passed on to the Handlbars template.
#[derive(Debug, Serialize)]
struct HandlebarsTableContext<'a> {
    name: &'a str,
    filter: Option<String>,
    columns: String,
    column_defaults: Vec<ColumnDefault<'a>>,
    sequence: Option<&'a SequenceConfig>,
}

#[derive(Debug, Serialize)]
struct ColumnDefault<'a> {
    column: &'a str,
    value: &'a str,
}

impl TableConfig {
    fn template_context<'a>(&'a self, name: &'a str) -> Option<HandlebarsTableContext<'a>> {
        let columns = self
            .columns
            .iter()
            .filter(|&(_, &vis)| vis == ColumnVisibility::Public)
            .map(|(col, _)| format!("\"{col}\""))
            .collect::<Vec<String>>()
            .join(", ");
        if columns.is_empty() {
            None
        } else {
            let filter = self.filter.as_ref().map(|s| s.replace('\n', " "));
            let column_defaults = self
                .column_defaults
                .iter()
                .map(|(k, v)| ColumnDefault {
                    column: k.as_str(),
                    value: v.as_str(),
                })
                .collect();
            Some(HandlebarsTableContext {
                name,
                filter,
                columns,
                column_defaults,
                sequence: self.sequence.as_ref(),
            })
        }
    }
}

/// Subset of the configuration data to be passed on to the Handlbars template.
#[derive(Debug, Serialize)]
struct TemplateContext<'a> {
    tables: Vec<HandlebarsTableContext<'a>>,
}

impl VisibilityConfig {
    fn template_context(&self) -> TemplateContext<'_> {
        let tables = self
            .topological_sort()
            .into_iter()
            .filter_map(|table| self.0[table].template_context(table))
            .collect();
        TemplateContext { tables }
    }

    fn gen_psql_scripts<W>(&self, mut export_writer: W, mut import_writer: W) -> anyhow::Result<()>
    where
        W: std::io::Write,
    {
        use minijinja::Environment;

        let mut env = Environment::new();
        env.add_template("dump-export.sql", include_str!("dump-export.sql.j2"))
            .context("Failed to load dump-export.sql.j2 template")?;
        env.add_template("dump-import.sql", include_str!("dump-import.sql.j2"))
            .context("Failed to load dump-import.sql.j2 template")?;

        let context = self.template_context();

        debug!("Rendering dump-export.sql file…");
        let export_sql = env
            .get_template("dump-export.sql")
            .unwrap()
            .render(&context)
            .context("Failed to render dump-export.sql file")?;

        debug!("Rendering dump-import.sql file…");
        let import_sql = env
            .get_template("dump-import.sql")
            .unwrap()
            .render(&context)
            .context("Failed to render dump-import.sql file")?;

        debug!("Writing dump-export.sql file…");
        export_writer
            .write_all(export_sql.as_bytes())
            .context("Failed to write dump-export.sql file")?;

        debug!("Writing dump-import.sql file…");
        import_writer
            .write_all(import_sql.as_bytes())
            .context("Failed to write dump-import.sql file")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crates_io_test_db::TestDatabase;
    use diesel::prelude::*;
    use diesel_async::{AsyncPgConnection, RunQueryDsl};
    use std::collections::HashSet;
    use std::iter::FromIterator;

    /// Test whether the visibility configuration matches the schema of the
    /// test database.
    #[tokio::test]
    async fn check_visibility_config() {
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        let db_columns = HashSet::<Column>::from_iter(get_db_columns(&mut conn).await);
        let vis_columns = VisibilityConfig::get()
            .0
            .iter()
            .flat_map(|(table, config)| {
                config.columns.keys().map(|column| Column {
                    table_name: table.clone(),
                    column_name: column.clone(),
                })
            })
            .collect();
        let mut errors = vec![];
        for Column {
            table_name,
            column_name,
        } in db_columns.difference(&vis_columns)
        {
            errors.push(format!(
                "No visibility information for columns {table_name}.{column_name}."
            ));
        }
        for Column {
            table_name,
            column_name,
        } in vis_columns.difference(&db_columns)
        {
            errors.push(format!(
                "Column {table_name}.{column_name} does not exist in the database."
            ));
        }
        assert!(
            errors.is_empty(),
            "The visibility configuration does not match the database schema:\n{}",
            errors.join("\n"),
        );
    }

    mod information_schema {
        use diesel::table;

        table! {
            information_schema.columns (table_schema, table_name, column_name) {
                table_schema -> Text,
                table_name -> Text,
                column_name -> Text,
                ordinal_position -> Integer,
            }
        }
    }

    #[derive(Debug, Eq, Hash, PartialEq, Queryable)]
    struct Column {
        table_name: String,
        column_name: String,
    }

    async fn get_db_columns(conn: &mut AsyncPgConnection) -> Vec<Column> {
        use information_schema::columns;
        columns::table
            .select((columns::table_name, columns::column_name))
            .filter(columns::table_schema.eq("public"))
            .order_by((columns::table_name, columns::ordinal_position))
            .load(conn)
            .await
            .unwrap()
    }
}
