use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};
use claims::{assert_ok, assert_some};
use crates_io::schema::versions;
use diesel::QueryDsl;
use diesel_async::RunQueryDsl;
use insta::{assert_json_snapshot, assert_snapshot};

const README_CONTENT: &str =
    "# Analysis Test Crate\n\nThis is a test crate for linecount analysis.";

const MAIN_RS_CONTENT: &str = r#"//! Main binary for analysis test crate
//!
//! This is a simple hello world program.

fn main() {
    println!("Hello, world!");
    let x = 42;
    let y = x + 1;
    println!("The answer is: {}", y);
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
"#;

const LIB_RS_CONTENT: &str = r#"//! Analysis test library
//!
//! Contains utility functions for testing.

/// Adds two numbers together
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Multiplies two numbers
pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
    }

    #[test]
    fn test_multiply() {
        assert_eq!(multiply(4, 5), 20);
    }
}
"#;

const INTEGRATION_TEST_CONTENT: &str = r#"//! Integration tests for analysis_test crate

use analysis_test::{add, multiply};

#[test]
fn integration_test_add() {
    assert_eq!(add(10, 15), 25);
}

#[test]
fn integration_test_multiply() {
    assert_eq!(multiply(3, 7), 21);
}

#[test]
fn combined_operations() {
    let result = multiply(add(2, 3), 4);
    assert_eq!(result, 20);
}
"#;

#[tokio::test(flavor = "multi_thread")]
async fn test_crate_files_are_analyzed() {
    let (app, _, _, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;

    // Create a crate with multiple source files of different types
    let crate_to_publish = PublishBuilder::new("analysis_test", "1.0.0")
        .readme(README_CONTENT)
        .add_file("analysis_test-1.0.0/src/main.rs", MAIN_RS_CONTENT)
        .add_file("analysis_test-1.0.0/src/lib.rs", LIB_RS_CONTENT)
        .add_file(
            "analysis_test-1.0.0/tests/integration_test.rs",
            INTEGRATION_TEST_CONTENT,
        );

    let response = token.publish_crate(crate_to_publish).await;
    assert_snapshot!(response.status(), @"200 OK");

    let result = versions::table
        .select(versions::linecounts)
        .first(&mut conn)
        .await;

    let linecount_data: Option<serde_json::Value> = assert_ok!(result);
    let linecount_data = assert_some!(linecount_data);
    assert_json_snapshot!(linecount_data);
}
