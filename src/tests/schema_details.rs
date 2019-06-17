use diesel::prelude::*;
use diesel::sql_types::Text;

use crate::TestApp;

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
                constraint.name, row.table_name
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
                constraint.name, row.table_name
            );
        }
    }
}

#[derive(QueryableByName)]
struct FkConstraint {
    #[sql_type = "Text"]
    #[column_name = "conname"]
    name: String,
    #[sql_type = "Text"]
    definition: String,
}

#[derive(QueryableByName)]
struct TableNameAndConstraint {
    #[sql_type = "Text"]
    #[column_name = "relname"]
    table_name: String,
    #[diesel(embed)]
    constraint: Option<FkConstraint>,
}

fn get_fk_constraint_definitions(column_name: &str) -> Vec<TableNameAndConstraint> {
    use diesel::sql_query;

    let (app, _) = TestApp::init().empty();

    app.db(|conn| {
        sql_query(include_str!("load_foreign_key_constraints.sql"))
            .bind::<Text, _>(column_name)
            .load(conn)
            .unwrap()
    })
}
