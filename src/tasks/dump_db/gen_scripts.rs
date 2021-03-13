use std::{collections::BTreeMap, fs::File, path::Path};

use crate::tasks::dump_db::configuration::{ColumnVisibility, TableConfig, VisibilityConfig};
use swirl::PerformError;

pub fn gen_scripts(export_script: &Path, import_script: &Path) -> Result<(), PerformError> {
    let config = VisibilityConfig::get();
    let export_sql = File::create(export_script)?;
    let import_sql = File::create(import_script)?;
    config.gen_psql_scripts(export_sql, import_sql)
}

/// Subset of the configuration data to be passed on to the Handlbars template.
#[derive(Debug, Serialize)]
struct HandlebarsTableContext<'a> {
    name: &'a str,
    filter: Option<String>,
    columns: String,
    column_defaults: BTreeMap<&'a str, &'a str>,
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
            let filter = self.filter.as_ref().map(|s| s.replace('\n', " "));
            let column_defaults = self
                .column_defaults
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            Some(HandlebarsTableContext {
                name,
                filter,
                columns,
                column_defaults,
            })
        }
    }
}

/// Subset of the configuration data to be passed on to the Handlbars template.
#[derive(Debug, Serialize)]
struct HandlebarsContext<'a> {
    tables: Vec<HandlebarsTableContext<'a>>,
}

impl VisibilityConfig {
    fn handlebars_context(&self) -> HandlebarsContext<'_> {
        let tables = self
            .topological_sort()
            .into_iter()
            .filter_map(|table| self.0[table].handlebars_context(table))
            .collect();
        HandlebarsContext { tables }
    }

    fn gen_psql_scripts<W>(&self, export_sql: W, import_sql: W) -> Result<(), PerformError>
    where
        W: std::io::Write,
    {
        let context = self.handlebars_context();
        let mut handlebars = handlebars::Handlebars::new();
        handlebars.register_escape_fn(handlebars::no_escape);
        handlebars.render_template_to_write(
            include_str!("dump-export.sql.hbs"),
            &context,
            export_sql,
        )?;
        handlebars.render_template_to_write(
            include_str!("dump-import.sql.hbs"),
            &context,
            import_sql,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::pg_connection;
    use diesel::prelude::*;
    use std::collections::HashSet;
    use std::iter::FromIterator;

    /// Test whether the visibility configuration matches the schema of the
    /// test database.
    #[test]
    fn check_visibility_config() {
        let conn = pg_connection();
        let db_columns = HashSet::<Column>::from_iter(get_db_columns(&conn));
        let vis_columns = VisibilityConfig::get()
            .0
            .iter()
            .flat_map(|(table, config)| {
                config.columns.iter().map(move |(column, _)| Column {
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
                "No visibility information for columns {}.{}.",
                table_name, column_name
            ));
        }
        for Column {
            table_name,
            column_name,
        } in vis_columns.difference(&db_columns)
        {
            errors.push(format!(
                "Column {}.{} does not exist in the database.",
                table_name, column_name
            ));
        }
        assert!(
            errors.is_empty(),
            "The visibility configuration does not match the database schema:\n{}",
            errors.join("\n"),
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

    #[derive(Debug, Eq, Hash, PartialEq, Queryable)]
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
