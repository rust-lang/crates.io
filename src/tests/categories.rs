use crate::schema::categories;
use crates_io_test_db::TestDatabase;
use diesel::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

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

async fn select_slugs(conn: &mut AsyncPgConnection) -> Vec<String> {
    categories::table
        .select(categories::slug)
        .order(categories::slug)
        .load(conn)
        .await
        .unwrap()
}

#[tokio::test]
async fn sync_adds_new_categories() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    crate::boot::categories::sync_with_connection(ALGORITHMS_AND_SUCH, &mut conn)
        .await
        .unwrap();

    let categories = select_slugs(&mut conn).await;
    assert_eq!(categories, vec!["algorithms", "algorithms::such"]);
}

#[tokio::test]
async fn sync_removes_missing_categories() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    crate::boot::categories::sync_with_connection(ALGORITHMS_AND_SUCH, &mut conn)
        .await
        .unwrap();
    crate::boot::categories::sync_with_connection(ALGORITHMS, &mut conn)
        .await
        .unwrap();

    let categories = select_slugs(&mut conn).await;
    assert_eq!(categories, vec!["algorithms"]);
}

#[tokio::test]
async fn sync_adds_and_removes() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    crate::boot::categories::sync_with_connection(ALGORITHMS_AND_SUCH, &mut conn)
        .await
        .unwrap();
    crate::boot::categories::sync_with_connection(ALGORITHMS_AND_ANOTHER, &mut conn)
        .await
        .unwrap();

    let categories = select_slugs(&mut conn).await;
    assert_eq!(categories, vec!["algorithms", "another"]);
}

#[tokio::test]
async fn test_real_categories() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    const TOML: &str = include_str!("../boot/categories.toml");
    assert_ok!(crate::boot::categories::sync_with_connection(TOML, &mut conn).await);
}
