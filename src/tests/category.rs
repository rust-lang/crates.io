use conduit::{Handler, Method};

use cargo_registry::category::EncodableCategory;

#[derive(RustcDecodable)]
struct CategoryList { categories: Vec<EncodableCategory>, meta: CategoryMeta }
#[derive(RustcDecodable)]
struct CategoryMeta { total: i32 }

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
