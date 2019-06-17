use crate::{app, builders::CrateBuilder, new_category, new_user, req, RequestHelper, TestApp};
use cargo_registry::{
    models::Category,
    views::{EncodableCategory, EncodableCategoryWithSubcategories},
};

use conduit::{Handler, Method};

#[derive(Deserialize)]
struct CategoryList {
    categories: Vec<EncodableCategory>,
    meta: CategoryMeta,
}
#[derive(Deserialize)]
struct CategoryMeta {
    total: i32,
}
#[derive(Deserialize)]
struct GoodCategory {
    category: EncodableCategory,
}
#[derive(Deserialize)]
struct CategoryWithSubcategories {
    category: EncodableCategoryWithSubcategories,
}

#[test]
fn index() {
    let (app, anon) = TestApp::init().empty();
    let url = "/api/v1/categories";

    // List 0 categories if none exist
    let json: CategoryList = anon.get(url).good();
    assert_eq!(json.categories.len(), 0);
    assert_eq!(json.meta.total, 0);

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
    let json: CategoryList = anon.get(url).good();
    assert_eq!(json.categories.len(), 1);
    assert_eq!(json.meta.total, 1);
    assert_eq!(json.categories[0].category, "foo");
}

#[test]
fn show() {
    let (app, anon) = TestApp::init().empty();
    let url = "/api/v1/categories/foo-bar";

    // Return not found if a category doesn't exist
    anon.get(url).assert_not_found();

    // Create a category and a subcategory
    app.db(|conn| {
        t!(new_category("Foo Bar", "foo-bar", "Foo Bar crates").create_or_update(conn));
        t!(new_category("Foo Bar::Baz", "foo-bar::baz", "Baz crates").create_or_update(conn));
    });

    // The category and its subcategories should be in the json
    let json: CategoryWithSubcategories = anon.get(url).good();
    assert_eq!(json.category.category, "Foo Bar");
    assert_eq!(json.category.slug, "foo-bar");
    assert_eq!(json.category.subcategories.len(), 1);
    assert_eq!(json.category.subcategories[0].category, "Baz");
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn update_crate() {
    let (app, middle) = app();
    let mut req = req(Method::Get, "/api/v1/categories/foo");
    macro_rules! cnt {
        ($req:expr, $cat:expr) => {{
            $req.with_path(&format!("/api/v1/categories/{}", $cat));
            let mut response = ok_resp!(middle.call($req));
            crate::json::<GoodCategory>(&mut response)
                .category
                .crates_cnt as usize
        }};
    }

    let krate = {
        let conn = t!(app.diesel_database.get());
        let user = t!(new_user("foo").create_or_update(&conn));
        t!(new_category("cat1", "cat1", "Category 1 crates").create_or_update(&conn));
        t!(new_category("Category 2", "category-2", "Category 2 crates").create_or_update(&conn));
        CrateBuilder::new("foo_crate", user.id).expect_build(&conn)
    };

    // Updating with no categories has no effect
    Category::update_crate(&app.diesel_database.get().unwrap(), &krate, &[]).unwrap();
    assert_eq!(cnt!(&mut req, "cat1"), 0);
    assert_eq!(cnt!(&mut req, "category-2"), 0);

    // Happy path adding one category
    Category::update_crate(&app.diesel_database.get().unwrap(), &krate, &["cat1"]).unwrap();
    assert_eq!(cnt!(&mut req, "cat1"), 1);
    assert_eq!(cnt!(&mut req, "category-2"), 0);

    // Replacing one category with another
    Category::update_crate(&app.diesel_database.get().unwrap(), &krate, &["category-2"]).unwrap();
    assert_eq!(cnt!(&mut req, "cat1"), 0);
    assert_eq!(cnt!(&mut req, "category-2"), 1);

    // Removing one category
    Category::update_crate(&app.diesel_database.get().unwrap(), &krate, &[]).unwrap();
    assert_eq!(cnt!(&mut req, "cat1"), 0);
    assert_eq!(cnt!(&mut req, "category-2"), 0);

    // Adding 2 categories
    Category::update_crate(
        &app.diesel_database.get().unwrap(),
        &krate,
        &["cat1", "category-2"],
    )
    .unwrap();
    assert_eq!(cnt!(&mut req, "cat1"), 1);
    assert_eq!(cnt!(&mut req, "category-2"), 1);

    // Removing all categories
    Category::update_crate(&app.diesel_database.get().unwrap(), &krate, &[]).unwrap();
    assert_eq!(cnt!(&mut req, "cat1"), 0);
    assert_eq!(cnt!(&mut req, "category-2"), 0);

    // Attempting to add one valid category and one invalid category
    let invalid_categories = Category::update_crate(
        &app.diesel_database.get().unwrap(),
        &krate,
        &["cat1", "catnope"],
    )
    .unwrap();
    assert_eq!(invalid_categories, vec!["catnope"]);
    assert_eq!(cnt!(&mut req, "cat1"), 1);
    assert_eq!(cnt!(&mut req, "category-2"), 0);

    // Does not add the invalid category to the category list
    // (unlike the behavior of keywords)
    req.with_path("/api/v1/categories");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: CategoryList = crate::json(&mut response);
    assert_eq!(json.categories.len(), 2);
    assert_eq!(json.meta.total, 2);

    // Attempting to add a category by display text; must use slug
    Category::update_crate(&app.diesel_database.get().unwrap(), &krate, &["Category 2"]).unwrap();
    assert_eq!(cnt!(&mut req, "cat1"), 0);
    assert_eq!(cnt!(&mut req, "category-2"), 0);

    // Add a category and its subcategory
    {
        let conn = t!(app.diesel_database.get());
        t!(new_category("cat1::bar", "cat1::bar", "bar crates").create_or_update(&conn,));
        Category::update_crate(&conn, &krate, &["cat1", "cat1::bar"]).unwrap();
    }
    assert_eq!(cnt!(&mut req, "cat1"), 1);
    assert_eq!(cnt!(&mut req, "cat1::bar"), 1);
    assert_eq!(cnt!(&mut req, "category-2"), 0);
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

    #[derive(Deserialize, Debug, PartialEq)]
    struct Slug {
        id: String,
        slug: String,
        description: String,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct Slugs {
        category_slugs: Vec<Slug>,
    }

    let response = anon.get("/api/v1/category_slugs").good();
    let expected_response = Slugs {
        category_slugs: vec![
            Slug {
                id: "bar".into(),
                slug: "bar".into(),
                description: "For crates that bar".into(),
            },
            Slug {
                id: "foo".into(),
                slug: "foo".into(),
                description: "For crates that foo".into(),
            },
        ],
    };
    assert_eq!(expected_response, response);
}
