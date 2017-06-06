use conduit::{Handler, Method};

use cargo_registry::category::{Category, EncodableCategory, EncodableCategoryWithSubcategories};

#[derive(RustcDecodable)]
struct CategoryList {
    categories: Vec<EncodableCategory>,
    meta: CategoryMeta,
}
#[derive(RustcDecodable)]
struct CategoryMeta {
    total: i32,
}
#[derive(RustcDecodable)]
struct GoodCategory {
    category: EncodableCategory,
}
#[derive(RustcDecodable)]
struct CategoryWithSubcategories {
    category: EncodableCategoryWithSubcategories,
}

#[test]
fn index() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Get, "/api/v1/categories");

    // List 0 categories if none exist
    let mut response = ok_resp!(middle.call(&mut req));
    let json: CategoryList = ::json(&mut response);
    assert_eq!(json.categories.len(), 0);
    assert_eq!(json.meta.total, 0);

    {
        let conn = app.diesel_database.get().unwrap();
        // Create a category and a subcategory
        ::new_category("foo", "foo").find_or_create(&conn).unwrap();
        ::new_category("foo::bar", "foo::bar")
            .find_or_create(&conn)
            .unwrap();
    }

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
    let mut req = ::req(app.clone(), Method::Get, "/api/v1/categories/foo-bar");
    let response = t_resp!(middle.call(&mut req));
    assert_eq!(response.status.0, 404);

    {
        let conn = app.diesel_database.get().unwrap();
        ::new_category("Foo Bar", "foo-bar")
            .find_or_create(&conn)
            .unwrap();
        ::new_category("Foo Bar::Baz", "foo-bar::baz")
            .find_or_create(&conn)
            .unwrap();
    }

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
    let mut req = ::req(app.clone(), Method::Get, "/api/v1/categories/foo");
    macro_rules! cnt {
        ($req: expr, $cat: expr) => {{
            $req.with_path(&format!("/api/v1/categories/{}", $cat));
            let mut response = ok_resp!(middle.call($req));
            ::json::<GoodCategory>(&mut response).category.crates_cnt as usize
        }}
    }
    let krate;
    {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("foo").create_or_update(&conn).unwrap();
        krate = ::CrateBuilder::new("foocat", u.id).expect_build(&conn);
        ::new_category("cat1", "cat1")
            .find_or_create(&conn)
            .unwrap();
        ::new_category("Category 2", "category-2")
            .find_or_create(&conn)
            .unwrap();
    }

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
    ).unwrap();
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
    ).unwrap();
    assert_eq!(invalid_categories, vec!["catnope"]);
    assert_eq!(cnt!(&mut req, "cat1"), 1);
    assert_eq!(cnt!(&mut req, "category-2"), 0);

    // Does not add the invalid category to the category list
    // (unlike the behavior of keywords)
    req.with_path("/api/v1/categories");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: CategoryList = ::json(&mut response);
    assert_eq!(json.categories.len(), 2);
    assert_eq!(json.meta.total, 2);

    // Attempting to add a category by display text; must use slug
    Category::update_crate(&app.diesel_database.get().unwrap(), &krate, &["Category 2"]).unwrap();
    assert_eq!(cnt!(&mut req, "cat1"), 0);
    assert_eq!(cnt!(&mut req, "category-2"), 0);

    // Add a category and its subcategory
    {
        let conn = app.diesel_database.get().unwrap();
        ::new_category("cat1::bar", "cat1::bar")
            .find_or_create(&conn)
            .unwrap();
        Category::update_crate(&conn, &krate, &["cat1", "cat1::bar"]).unwrap();
    }
    assert_eq!(cnt!(&mut req, "cat1"), 1);
    assert_eq!(cnt!(&mut req, "cat1::bar"), 1);
    assert_eq!(cnt!(&mut req, "category-2"), 0);
}
