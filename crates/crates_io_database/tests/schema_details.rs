use crates_io_test_db::TestDatabase;
use diesel::prelude::*;
use diesel::sql_types::Text;
use diesel_async::RunQueryDsl;

#[tokio::test]
async fn all_columns_called_crate_id_have_a_cascading_foreign_key() {
    for row in get_fk_constraint_definitions("crate_id").await {
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

#[tokio::test]
async fn all_columns_called_version_id_have_a_cascading_foreign_key() {
    for row in get_fk_constraint_definitions("version_id").await {
        let constraint = match row.constraint {
            Some(c) => c,
            None => panic!(
                "Column called version_id on {} has no foreign key",
                row.table_name
            ),
        };

        if row.table_name == "default_versions" {
            // We explicitly don't want to enforce this on the default_versions table.
            continue;
        }

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
    #[diesel(sql_type = Text)]
    #[diesel(column_name = conname)]
    name: String,
    #[diesel(sql_type = Text)]
    definition: String,
}

#[derive(QueryableByName)]
struct TableNameAndConstraint {
    #[diesel(sql_type = Text)]
    #[diesel(column_name = relname)]
    table_name: String,
    #[diesel(embed)]
    constraint: Option<FkConstraint>,
}

async fn get_fk_constraint_definitions(column_name: &str) -> Vec<TableNameAndConstraint> {
    use diesel::sql_query;

    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    sql_query(include_str!("load_foreign_key_constraints.sql"))
        .bind::<Text, _>(column_name)
        .load(&mut conn)
        .await
        .unwrap()
}
