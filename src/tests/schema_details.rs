use diesel::prelude::*;

#[test]
fn all_columns_called_crate_id_have_a_cascading_foreign_key() {
    for (table_name, constraint) in get_fk_constraint_definitions("crate_id") {
        let constraint = match constraint {
            Some(c) => c,
            None => panic!(
                "Column called crate_id on {} has no foreign key",
                table_name
            ),
        };
        if !constraint.definition.contains("ON DELETE CASCADE") {
            panic!(
                "Foreign key {} on table {} should have `ON DELETE CASCADE` \
                 but it doesn't.",
                constraint.name,
                table_name
            );
        }
    }
}

#[test]
fn all_columns_called_version_id_have_a_cascading_foreign_key() {
    for (table_name, constraint) in get_fk_constraint_definitions("version_id") {
        let constraint = match constraint {
            Some(c) => c,
            None => panic!(
                "Column called version_id on {} has no foreign key",
                table_name
            ),
        };
        if !constraint.definition.contains("ON DELETE CASCADE") {
            panic!(
                "Foreign key {} on table {} should have `ON DELETE CASCADE` \
                 but it doesn't.",
                constraint.name,
                table_name
            );
        }
    }
}

#[derive(Queryable)]
struct FkConstraint {
    name: String,
    definition: String,
}

fn get_fk_constraint_definitions(column_name: &str) -> Vec<(String, Option<FkConstraint>)> {
    use diesel::expression::dsl::sql;
    use diesel::types::Text;

    let (_r, app, _) = ::app();
    let conn = app.diesel_database.get().unwrap();

    sql(include_str!("load_foreign_key_constraints.sql"))
        .bind::<Text, _>(column_name)
        .load(&*conn)
        .unwrap()
}
