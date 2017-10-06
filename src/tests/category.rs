use conduit::{Handler, Method};

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
    let mut req = ::req(app.clone(), Method::Get, "/api/v1/categories");

    // List 0 categories if none exist
    let mut response = ok_resp!(middle.call(&mut req));
    let json: CategoryList = ::json(&mut response);
    assert_eq!(json.categories.len(), 0);
    assert_eq!(json.meta.total, 0);

    // Create a category and a subcategory
    {
        let conn = t!(app.diesel_database.get());
        ::new_category("foo", "foo")
            .create_or_update(&conn)
            .unwrap();
        ::new_category("foo::bar", "foo::bar")
            .create_or_update(&conn)
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

    // Create a category and a subcategory
    {
        let conn = t!(app.diesel_database.get());

        t!(::new_category("Foo Bar", "foo-bar").create_or_update(&conn));
        t!(::new_category("Foo Bar::Baz", "foo-bar::baz").create_or_update(&conn));
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

    let krate = {
        let conn = t!(app.diesel_database.get());
        let user = t!(::new_user("foo").create_or_update(&conn));
        t!(::new_category("cat1", "cat1").create_or_update(&conn));
        t!(::new_category("Category 2", "category-2").create_or_update(&conn));
        ::CrateBuilder::new("foo_crate", user.id).expect_build(&conn)
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
        let conn = t!(app.diesel_database.get());
        t!(::new_category("cat1::bar", "cat1::bar").create_or_update(&conn,));
        Category::update_crate(&conn, &krate, &["cat1", "cat1::bar"]).unwrap();
    }
    assert_eq!(cnt!(&mut req, "cat1"), 1);
    assert_eq!(cnt!(&mut req, "cat1::bar"), 1);
    assert_eq!(cnt!(&mut req, "category-2"), 0);
}

#[test]
fn category_slugs_returns_all_slugs_in_alphabetical_order() {
    let (_b, app, middle) = ::app();
    {
        let conn = app.diesel_database.get().unwrap();
        ::new_category("Foo", "foo")
            .create_or_update(&conn)
            .unwrap();
        ::new_category("Bar", "bar")
            .create_or_update(&conn)
            .unwrap();
    }

    let mut req = ::req(app, Method::Get, "/api/v1/category_slugs");

    #[derive(Deserialize, Debug, PartialEq)]
    struct Slug {
        id: String,
        slug: String,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct Slugs {
        category_slugs: Vec<Slug>,
    }

    let response = ::json(&mut ok_resp!(middle.call(&mut req)));
    let expected_response = Slugs {
        category_slugs: vec![
            Slug {
                id: "bar".into(),
                slug: "bar".into(),
            },
            Slug {
                id: "foo".into(),
                slug: "foo".into(),
            },
        ],
    };
    assert_eq!(expected_response, response);
}
