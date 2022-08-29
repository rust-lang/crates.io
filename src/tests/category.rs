use crate::util::insta::assert_yaml_snapshot;
use crate::{
    builders::CrateBuilder, new_category, util::MockAnonymousUser, RequestHelper, TestApp,
};
use cargo_registry::models::Category;
use serde_json::Value;

#[test]
fn index() {
    let (app, anon) = TestApp::init().empty();

    // List 0 categories if none exist
    let json: Value = anon.get("/api/v1/categories").good();
    assert_yaml_snapshot!(json);

    // Create a category and a subcategory
    app.db(|conn| {
        new_category("foo", "foo", "Foo crates")
            .create_or_update(conn)
            .unwrap();
        new_category("foo::bar", "foo::bar", "Bar crates")
            .create_or_update(conn)
            .unwrap();
    });

    // Only the top-level categories should be on the page
    let json: Value = anon.get("/api/v1/categories").good();
    assert_yaml_snapshot!(json, {
        ".categories[].created_at" => "[datetime]",
    });
}

#[test]
fn show() {
    let (app, anon) = TestApp::init().empty();
    let url = "/api/v1/categories/foo-bar";

    // Return not found if a category doesn't exist
    anon.get(url).assert_not_found();

    // Create a category and a subcategory
    app.db(|conn| {
        assert_ok!(new_category("Foo Bar", "foo-bar", "Foo Bar crates").create_or_update(conn));
        assert_ok!(
            new_category("Foo Bar::Baz", "foo-bar::baz", "Baz crates").create_or_update(conn)
        );
    });

    // The category and its subcategories should be in the json
    let json: Value = anon.get(url).good();
    assert_yaml_snapshot!(json, {
        ".**.created_at" => "[datetime]",
    });
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn update_crate() {
    // Convenience function to get the number of crates in a category
    fn count(anon: &MockAnonymousUser, category: &str) -> usize {
        let json = anon.show_category(category);
        json.category.crates_cnt as usize
    }

    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        assert_ok!(new_category("cat1", "cat1", "Category 1 crates").create_or_update(conn));
        assert_ok!(
            new_category("Category 2", "category-2", "Category 2 crates").create_or_update(conn)
        );
        let krate = CrateBuilder::new("foo_crate", user.id).expect_build(conn);

        // Updating with no categories has no effect
        Category::update_crate(conn, &krate, &[]).unwrap();
        assert_eq!(count(&anon, "cat1"), 0);
        assert_eq!(count(&anon, "category-2"), 0);

        // Happy path adding one category
        Category::update_crate(conn, &krate, &["cat1"]).unwrap();
        assert_eq!(count(&anon, "cat1"), 1);
        assert_eq!(count(&anon, "category-2"), 0);

        // Replacing one category with another
        Category::update_crate(conn, &krate, &["category-2"]).unwrap();
        assert_eq!(count(&anon, "cat1"), 0);
        assert_eq!(count(&anon, "category-2"), 1);

        // Removing one category
        Category::update_crate(conn, &krate, &[]).unwrap();
        assert_eq!(count(&anon, "cat1"), 0);
        assert_eq!(count(&anon, "category-2"), 0);

        // Adding 2 categories
        Category::update_crate(conn, &krate, &["cat1", "category-2"]).unwrap();
        assert_eq!(count(&anon, "cat1"), 1);
        assert_eq!(count(&anon, "category-2"), 1);

        // Removing all categories
        Category::update_crate(conn, &krate, &[]).unwrap();
        assert_eq!(count(&anon, "cat1"), 0);
        assert_eq!(count(&anon, "category-2"), 0);

        // Attempting to add one valid category and one invalid category
        let invalid_categories =
            Category::update_crate(conn, &krate, &["cat1", "catnope"]).unwrap();
        assert_eq!(invalid_categories, vec!["catnope"]);
        assert_eq!(count(&anon, "cat1"), 1);
        assert_eq!(count(&anon, "category-2"), 0);

        // Does not add the invalid category to the category list
        // (unlike the behavior of keywords)
        let json = anon.show_category_list();
        assert_eq!(json.categories.len(), 2);
        assert_eq!(json.meta.total, 2);

        // Attempting to add a category by display text; must use slug
        Category::update_crate(conn, &krate, &["Category 2"]).unwrap();
        assert_eq!(count(&anon, "cat1"), 0);
        assert_eq!(count(&anon, "category-2"), 0);

        // Add a category and its subcategory
        assert_ok!(new_category("cat1::bar", "cat1::bar", "bar crates").create_or_update(conn));
        Category::update_crate(conn, &krate, &["cat1", "cat1::bar"]).unwrap();

        assert_eq!(count(&anon, "cat1"), 1);
        assert_eq!(count(&anon, "cat1::bar"), 1);
        assert_eq!(count(&anon, "category-2"), 0);
    });
}

#[test]
fn category_slugs_returns_all_slugs_in_alphabetical_order() {
    let (app, anon) = TestApp::init().empty();
    app.db(|conn| {
        new_category("Foo", "foo", "For crates that foo")
            .create_or_update(conn)
            .unwrap();
        new_category("Bar", "bar", "For crates that bar")
            .create_or_update(conn)
            .unwrap();
    });

    let response: Value = anon.get("/api/v1/category_slugs").good();
    assert_yaml_snapshot!(response);
}
