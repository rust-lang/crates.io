use conduit::{Handler, Method};
use conduit_test::MockRequest;

use cargo_registry::db::RequestTransaction;
use cargo_registry::category::{Category, EncodableCategory, EncodableCategoryWithSubcategories};

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
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/categories");

    // List 0 categories if none exist
    let mut response = ok_resp!(middle.call(&mut req));
    let json: CategoryList = ::json(&mut response);
    assert_eq!(json.categories.len(), 0);
    assert_eq!(json.meta.total, 0);

    // Create a category and a subcategory
    ::mock_category(&mut req, "foo", "foo");
    ::mock_category(&mut req, "foo::bar", "foo::bar");

    let mut response = ok_resp!(middle.call(&mut req));
    let json: CategoryList = ::json(&mut response);

    // Only the top-level categories should be on the page
    assert_eq!(json.categories.len(), 1);
    assert_eq!(json.meta.total, 1);
    assert_eq!(json.categories[0].category, "foo");
}

#[test]
fn show() {
    let (_b, app, middle) = ::app();

    // Return not found if a category doesn't exist
    let mut req = ::req(app, Method::Get, "/api/v1/categories/foo-bar");
    let response = t_resp!(middle.call(&mut req));
    assert_eq!(response.status.0, 404);

    // Create a category and a subcategory
    ::mock_category(&mut req, "Foo Bar", "foo-bar");
    ::mock_category(&mut req, "Foo Bar::Baz", "foo-bar::baz");

    // The category and its subcategories should be in the json
    let mut response = ok_resp!(middle.call(&mut req));
    let json: CategoryWithSubcategories = ::json(&mut response);
    assert_eq!(json.category.category, "Foo Bar");
    assert_eq!(json.category.slug, "foo-bar");
    assert_eq!(json.category.subcategories.len(), 1);
    assert_eq!(json.category.subcategories[0].category, "Foo Bar::Baz");
}

#[test]
fn update_crate() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/categories/foo");
    let cnt = |req: &mut MockRequest, cat: &str| {
        req.with_path(&format!("/api/v1/categories/{}", cat));
        let mut response = ok_resp!(middle.call(req));
        ::json::<GoodCategory>(&mut response).category.crates_cnt as usize
    };
    ::mock_user(&mut req, ::user("foo"));
    let (krate, _) = ::mock_crate(&mut req, ::krate("foocat"));
    ::mock_category(&mut req, "cat1", "cat1");
    ::mock_category(&mut req, "Category 2", "category-2");

    // Updating with no categories has no effect
    Category::update_crate_old(req.tx().unwrap(), &krate, &[]).unwrap();
    assert_eq!(cnt(&mut req, "cat1"), 0);
    assert_eq!(cnt(&mut req, "category-2"), 0);

    // Happy path adding one category
    Category::update_crate_old(req.tx().unwrap(), &krate, &["cat1".to_string()]).unwrap();
    assert_eq!(cnt(&mut req, "cat1"), 1);
    assert_eq!(cnt(&mut req, "category-2"), 0);

    // Replacing one category with another
    Category::update_crate_old(req.tx().unwrap(), &krate, &["category-2".to_string()]).unwrap();
    assert_eq!(cnt(&mut req, "cat1"), 0);
    assert_eq!(cnt(&mut req, "category-2"), 1);

    // Removing one category
    Category::update_crate_old(req.tx().unwrap(), &krate, &[]).unwrap();
    assert_eq!(cnt(&mut req, "cat1"), 0);
    assert_eq!(cnt(&mut req, "category-2"), 0);

    // Adding 2 categories
    Category::update_crate_old(
        req.tx().unwrap(),
        &krate,
        &["cat1".to_string(), "category-2".to_string()],
    ).unwrap();
    assert_eq!(cnt(&mut req, "cat1"), 1);
    assert_eq!(cnt(&mut req, "category-2"), 1);

    // Removing all categories
    Category::update_crate_old(req.tx().unwrap(), &krate, &[]).unwrap();
    assert_eq!(cnt(&mut req, "cat1"), 0);
    assert_eq!(cnt(&mut req, "category-2"), 0);

    // Attempting to add one valid category and one invalid category
    let invalid_categories = Category::update_crate_old(
        req.tx().unwrap(),
        &krate,
        &["cat1".to_string(), "catnope".to_string()],
    ).unwrap();
    assert_eq!(invalid_categories, vec!["catnope".to_string()]);
    assert_eq!(cnt(&mut req, "cat1"), 1);
    assert_eq!(cnt(&mut req, "category-2"), 0);

    // Does not add the invalid category to the category list
    // (unlike the behavior of keywords)
    req.with_path("/api/v1/categories");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: CategoryList = ::json(&mut response);
    assert_eq!(json.categories.len(), 2);
    assert_eq!(json.meta.total, 2);

    // Attempting to add a category by display text; must use slug
    Category::update_crate_old(req.tx().unwrap(), &krate, &["Category 2".to_string()]).unwrap();
    assert_eq!(cnt(&mut req, "cat1"), 0);
    assert_eq!(cnt(&mut req, "category-2"), 0);

    // Add a category and its subcategory
    ::mock_category(&mut req, "cat1::bar", "cat1::bar");
    Category::update_crate_old(
        req.tx().unwrap(),
        &krate,
        &["cat1".to_string(), "cat1::bar".to_string()],
    ).unwrap();
    assert_eq!(cnt(&mut req, "cat1"), 1);
    assert_eq!(cnt(&mut req, "cat1::bar"), 1);
    assert_eq!(cnt(&mut req, "category-2"), 0);
}
