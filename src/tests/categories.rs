use crates_io::schema::categories;
use crates_io_test_db::TestDatabase;
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

fn select_slugs(conn: &mut PgConnection) -> Vec<String> {
    categories::table
        .select(categories::slug)
        .order(categories::slug)
        .load(conn)
        .unwrap()
}

#[test]
fn sync_adds_new_categories() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.connect();

    ::crates_io::boot::categories::sync_with_connection(ALGORITHMS_AND_SUCH, &mut conn).unwrap();

    let categories = select_slugs(&mut conn);
    assert_eq!(categories, vec!["algorithms", "algorithms::such"]);
}

#[test]
fn sync_removes_missing_categories() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.connect();

    ::crates_io::boot::categories::sync_with_connection(ALGORITHMS_AND_SUCH, &mut conn).unwrap();
    ::crates_io::boot::categories::sync_with_connection(ALGORITHMS, &mut conn).unwrap();

    let categories = select_slugs(&mut conn);
    assert_eq!(categories, vec!["algorithms"]);
}

#[test]
fn sync_adds_and_removes() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.connect();

    ::crates_io::boot::categories::sync_with_connection(ALGORITHMS_AND_SUCH, &mut conn).unwrap();
    ::crates_io::boot::categories::sync_with_connection(ALGORITHMS_AND_ANOTHER, &mut conn).unwrap();

    let categories = select_slugs(&mut conn);
    assert_eq!(categories, vec!["algorithms", "another"]);
}

#[test]
fn test_real_categories() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.connect();

    const TOML: &str = include_str!("../boot/categories.toml");
    assert_ok!(crates_io::boot::categories::sync_with_connection(
        TOML, &mut conn
    ));
}
