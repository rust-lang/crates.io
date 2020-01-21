use std::{
    collections::{BTreeMap, VecDeque},
    fs::File,
    path::Path,
};

use swirl::PerformError;

pub fn gen_scripts(export_script: &Path, import_script: &Path) -> Result<(), PerformError> {
    let config: VisibilityConfig = toml::from_str(include_str!("dump-db.toml")).unwrap();
    let export_sql = File::create(export_script)?;
    let import_sql = File::create(import_script)?;
    config.gen_psql_scripts(export_sql, import_sql)
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
#[derive(Clone, Debug, Default, Deserialize)]
struct TableConfig {
    #[serde(default)]
    dependencies: Vec<String>,
    filter: Option<String>,
    columns: BTreeMap<String, ColumnVisibility>,
    #[serde(default)]
    column_defaults: BTreeMap<String, String>,
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

/// Maps table names to the respective configurations. Used to load `dump_db.toml`.
#[derive(Clone, Debug, Default, Deserialize)]
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
        let mut rev_deps: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for (table, config) in self.0.iter() {
            num_deps.insert(table.as_str(), config.dependencies.len());
            for dep in &config.dependencies {
                rev_deps
                    .entry(dep.as_str())
                    .or_default()
                    .push(table.as_str());
            }
        }
        let mut ready: VecDeque<&str> = num_deps
            .iter()
            .filter(|(_, &count)| count == 0)
            .map(|(&table, _)| table)
            .collect();
        while let Some(table) = ready.pop_front() {
            result.push(table);
            for dep in rev_deps.get(table).iter().cloned().flatten() {
                *num_deps.get_mut(dep).unwrap() -= 1;
                if num_deps[dep] == 0 {
                    ready.push_back(dep);
                }
            }
        }
        assert_eq!(
            self.0.len(),
            result.len(),
            "circular dependencies in database dump configuration detected",
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
        let vis_columns = toml::from_str::<VisibilityConfig>(include_str!("dump-db.toml"))
            .unwrap()
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

    fn table_config_with_deps(deps: &[&str]) -> TableConfig {
        TableConfig {
            dependencies: deps.iter().cloned().map(ToOwned::to_owned).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn test_topological_sort() {
        let mut config = VisibilityConfig::default();
        let tables = &mut config.0;
        tables.insert("a".to_owned(), table_config_with_deps(&["b", "c"]));
        tables.insert("b".to_owned(), table_config_with_deps(&["c", "d"]));
        tables.insert("c".to_owned(), table_config_with_deps(&["d"]));
        config.0.insert("d".to_owned(), table_config_with_deps(&[]));
        assert_eq!(config.topological_sort(), ["d", "c", "b", "a"]);
    }

    #[test]
    #[should_panic]
    fn topological_sort_panics_for_cyclic_dependency() {
        let mut config = VisibilityConfig::default();
        let tables = &mut config.0;
        tables.insert("a".to_owned(), table_config_with_deps(&["b"]));
        tables.insert("b".to_owned(), table_config_with_deps(&["a"]));
        config.topological_sort();
    }
}
