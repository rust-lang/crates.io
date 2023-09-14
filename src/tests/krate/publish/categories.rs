use crate::builders::PublishBuilder;
use crate::new_category;
use crate::util::{RequestHelper, TestApp};

#[test]
fn good_categories() {
    let (app, _, _, token) = TestApp::full().with_token();

    app.db(|conn| {
        new_category("Category 1", "cat1", "Category 1 crates")
            .create_or_update(conn)
            .unwrap();
    });

    let crate_to_publish = PublishBuilder::new("foo_good_cat", "1.0.0").category("cat1");
    let json = token.publish_crate(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_good_cat");
    assert_eq!(json.krate.max_version, "1.0.0");
    assert_eq!(json.warnings.invalid_categories.len(), 0);
}

#[test]
fn ignored_categories() {
    let (_, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_ignored_cat", "1.0.0").category("bar");
    let json = token.publish_crate(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_ignored_cat");
    assert_eq!(json.krate.max_version, "1.0.0");
    assert_eq!(json.warnings.invalid_categories, vec!["bar"]);
}
