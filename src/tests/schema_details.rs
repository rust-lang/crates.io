use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::query_source::QueryableByName;
use diesel::row::NamedRow;
use std::error::Error;

#[test]
fn all_columns_called_crate_id_have_a_cascading_foreign_key() {
    for row in get_fk_constraint_definitions("crate_id") {
        let constraint = match row.constraint {
            Some(c) => c,
            None => panic!(
                "Column called crate_id on {} has no foreign key",
                row.table_name
            ),
        };
        if !constraint.definition.contains("ON DELETE CASCADE") {
            panic!(
                "Foreign key {} on table {} should have `ON DELETE CASCADE` \
                 but it doesn't.",
                constraint.name,
                row.table_name
            );
        }
    }
}

#[test]
fn all_columns_called_version_id_have_a_cascading_foreign_key() {
    for row in get_fk_constraint_definitions("version_id") {
        let constraint = match row.constraint {
            Some(c) => c,
            None => panic!(
                "Column called version_id on {} has no foreign key",
                row.table_name
            ),
        };
        if !constraint.definition.contains("ON DELETE CASCADE") {
            panic!(
                "Foreign key {} on table {} should have `ON DELETE CASCADE` \
                 but it doesn't.",
                constraint.name,
                row.table_name
            );
        }
    }
}

struct FkConstraint {
    name: String,
    definition: String,
}

struct TableNameAndConstraint {
    table_name: String,
    constraint: Option<FkConstraint>,
}

impl QueryableByName<Pg> for TableNameAndConstraint {
    fn build<R: NamedRow<Pg>>(row: &R) -> Result<Self, Box<Error + Send + Sync>> {
        use diesel::types::{Nullable, Text};

        let constraint = match row.get::<Nullable<Text>, _>("conname")? {
            Some(name) => Some(FkConstraint {
                definition: row.get::<Text, _>("definition")?,
                name,
            }),
            None => None,
        };
        Ok(TableNameAndConstraint {
            table_name: row.get::<Text, _>("relname")?,
            constraint,
        })
    }
}

fn get_fk_constraint_definitions(column_name: &str) -> Vec<TableNameAndConstraint> {
    use diesel::sql_query;
    use diesel::types::Text;

    let (_r, app, _) = ::app();
    let conn = app.diesel_database.get().unwrap();

    sql_query(include_str!("load_foreign_key_constraints.sql"))
        .bind::<Text, _>(column_name)
        .load(&*conn)
        .unwrap()
}
