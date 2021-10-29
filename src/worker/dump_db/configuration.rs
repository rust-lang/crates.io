use std::collections::{BTreeMap, VecDeque};

/// An enum indicating whether a column is included in the database dumps.
/// Public columns are included, private are not.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(super) enum ColumnVisibility {
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
pub(super) struct TableConfig {
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub filter: Option<String>,
    pub columns: BTreeMap<String, ColumnVisibility>,
    #[serde(default)]
    pub column_defaults: BTreeMap<String, String>,
}

/// Maps table names to the respective configurations. Used to load `dump_db.toml`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(transparent)]
pub(super) struct VisibilityConfig(pub BTreeMap<String, TableConfig>);

impl VisibilityConfig {
    pub(super) fn get() -> Self {
        toml::from_str(include_str!("dump-db.toml")).unwrap()
    }

    /// Sort the tables in a way that dependencies come before dependent tables.
    ///
    /// Returns a vector of table names.
    pub(super) fn topological_sort(&self) -> Vec<&str> {
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
        let mut result = Vec::with_capacity(ready.len());
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
