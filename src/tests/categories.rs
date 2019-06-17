use cargo_registry::schema::categories;

use diesel::*;

const ALGORITHMS: &str = r#"
[algorithms]
name = "Algorithms"
description = """
Rust implementations of core algorithms such as hashing, sorting, \
searching, and more.\
""""#;

const ALGORITHMS_AND_SUCH: &str = r#"
[algorithms]
name = "Algorithms"
description = """
Rust implementations of core algorithms such as hashing, sorting, \
searching, and more.\
"""

[algorithms.categories.such]
name = "Such"
description = """
Other stuff
""""#;

const ALGORITHMS_AND_ANOTHER: &str = r#"
[algorithms]
name = "Algorithms"
description = """
Rust implementations of core algorithms such as hashing, sorting, \
searching, and more.\
"""

[another]
name = "Another"
description = "Another category ho hum"
"#;

fn pg_connection() -> PgConnection {
    let database_url =
        dotenv::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set to run tests");
    let conn = PgConnection::establish(&database_url).unwrap();
    conn.begin_test_transaction().unwrap();
    conn
}

fn select_slugs(conn: &PgConnection) -> Vec<String> {
    categories::table
        .select(categories::slug)
        .order(categories::slug)
        .load::<String>(conn)
        .unwrap()
}

#[test]
fn sync_adds_new_categories() {
    let conn = pg_connection();

    ::cargo_registry::boot::categories::sync_with_connection(ALGORITHMS_AND_SUCH, &conn).unwrap();

    let categories = select_slugs(&conn);
    assert_eq!(categories, vec!["algorithms", "algorithms::such"]);
}

#[test]
fn sync_removes_missing_categories() {
    let conn = pg_connection();

    ::cargo_registry::boot::categories::sync_with_connection(ALGORITHMS_AND_SUCH, &conn).unwrap();
    ::cargo_registry::boot::categories::sync_with_connection(ALGORITHMS, &conn).unwrap();

    let categories = select_slugs(&conn);
    assert_eq!(categories, vec!["algorithms"]);
}

#[test]
fn sync_adds_and_removes() {
    let conn = pg_connection();

    ::cargo_registry::boot::categories::sync_with_connection(ALGORITHMS_AND_SUCH, &conn).unwrap();
    ::cargo_registry::boot::categories::sync_with_connection(ALGORITHMS_AND_ANOTHER, &conn)
        .unwrap();

    let categories = select_slugs(&conn);
    assert_eq!(categories, vec!["algorithms", "another"]);
}
