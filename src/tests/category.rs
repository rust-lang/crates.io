use postgres::GenericConnection;
use conduit::{Handler, Request, Method};
use conduit_test::MockRequest;

use cargo_registry::db::RequestTransaction;
use cargo_registry::category::{Category, EncodableCategory};

#[derive(RustcDecodable)]
struct CategoryList { categories: Vec<EncodableCategory>, meta: CategoryMeta }
#[derive(RustcDecodable)]
struct CategoryMeta { total: i32 }
#[derive(RustcDecodable)]
struct GoodCategory { category: EncodableCategory }

#[test]
fn index() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/categories");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: CategoryList = ::json(&mut response);
    assert_eq!(json.categories.len(), 0);
    assert_eq!(json.meta.total, 0);

    ::mock_category(&mut req, "foo");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: CategoryList = ::json(&mut response);
    assert_eq!(json.categories.len(), 1);
    assert_eq!(json.meta.total, 1);
    assert_eq!(json.categories[0].category, "foo");
}

#[test]
fn show() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/categories/foo");
    let response = t_resp!(middle.call(&mut req));
    assert_eq!(response.status.0, 404);

    ::mock_category(&mut req, "foo");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: GoodCategory = ::json(&mut response);
    assert_eq!(json.category.category, "foo");
}

fn tx(req: &Request) -> &GenericConnection { req.tx().unwrap() }

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
    let (krate, _) = ::mock_crate(&mut req, ::krate("foo"));
    ::mock_category(&mut req, "cat1");
    ::mock_category(&mut req, "cat2");

    // Updating with no categories has no effect
    Category::update_crate(tx(&req), &krate, &[]).unwrap();
    assert_eq!(cnt(&mut req, "cat1"), 0);
    assert_eq!(cnt(&mut req, "cat2"), 0);

    // Happy path adding one category
    Category::update_crate(tx(&req), &krate, &["cat1".to_string()]).unwrap();
    assert_eq!(cnt(&mut req, "cat1"), 1);
    assert_eq!(cnt(&mut req, "cat2"), 0);

    // Replacing one category with another
    Category::update_crate(tx(&req), &krate, &["cat2".to_string()]).unwrap();
    assert_eq!(cnt(&mut req, "cat1"), 0);
    assert_eq!(cnt(&mut req, "cat2"), 1);

    // Removing one category
    Category::update_crate(tx(&req), &krate, &[]).unwrap();
    assert_eq!(cnt(&mut req, "cat1"), 0);
    assert_eq!(cnt(&mut req, "cat2"), 0);

    // Adding 2 categories
    Category::update_crate(tx(&req), &krate, &["cat1".to_string(),
                                               "cat2".to_string()]).unwrap();
    assert_eq!(cnt(&mut req, "cat1"), 1);
    assert_eq!(cnt(&mut req, "cat2"), 1);

    // Removing all categories
    Category::update_crate(tx(&req), &krate, &[]).unwrap();
    assert_eq!(cnt(&mut req, "cat1"), 0);
    assert_eq!(cnt(&mut req, "cat2"), 0);

    // Attempting to add one valid category and one invalid category
    Category::update_crate(tx(&req), &krate, &["cat1".to_string(),
                                               "catnope".to_string()]).unwrap();

    assert_eq!(cnt(&mut req, "cat1"), 1);
    assert_eq!(cnt(&mut req, "cat2"), 0);

    // Does not add the invalid category to the category list
    // (unlike the behavior of keywords)
    req.with_path("/api/v1/categories");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: CategoryList = ::json(&mut response);
    assert_eq!(json.categories.len(), 2);
    assert_eq!(json.meta.total, 2);
}
