use conduit::{Handler, Method};
use conduit_test::MockRequest;

use cargo_registry::db::RequestTransaction;
use cargo_registry::keyword::{Keyword, EncodableKeyword};

#[derive(RustcDecodable)]
struct KeywordList { keywords: Vec<EncodableKeyword>, meta: KeywordMeta }
#[derive(RustcDecodable)]
struct KeywordMeta { total: i32 }
#[derive(RustcDecodable)]
struct GoodKeyword { keyword: EncodableKeyword }

#[test]
fn index() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/keywords");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: KeywordList = ::json(&mut response);
    assert_eq!(json.keywords.len(), 0);
    assert_eq!(json.meta.total, 0);

    ::mock_keyword(&mut req, "foo");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: KeywordList = ::json(&mut response);
    assert_eq!(json.keywords.len(), 1);
    assert_eq!(json.meta.total, 1);
    assert_eq!(json.keywords[0].keyword, "foo".to_string());
}

#[test]
fn show() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/keywords/foo");
    let response = t_resp!(middle.call(&mut req));
    assert_eq!(response.status.0, 404);

    ::mock_keyword(&mut req, "foo");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: GoodKeyword = ::json(&mut response);
    assert_eq!(json.keyword.keyword, "foo".to_string());
}

#[test]
fn uppercase() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/keywords/UPPER");
    ::mock_keyword(&mut req, "UPPER");

    let mut res = ok_resp!(middle.call(&mut req));
    let json: GoodKeyword = ::json(&mut res);
    assert_eq!(json.keyword.keyword, "upper".to_string());
}

#[test]
fn update_crate() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/keywords/foo");
    let cnt = |req: &mut MockRequest, kw: &str| {
        req.with_path(&format!("/api/v1/keywords/{}", kw));
        let mut response = ok_resp!(middle.call(req));
        ::json::<GoodKeyword>(&mut response).keyword.crates_cnt as usize
    };
    ::mock_user(&mut req, ::user("foo"));
    let (krate, _) = ::mock_crate(&mut req, ::krate("fookey"));
    ::mock_keyword(&mut req, "kw1");
    ::mock_keyword(&mut req, "kw2");

    Keyword::update_crate_old(req.tx().unwrap(), &krate, &[]).unwrap();
    assert_eq!(cnt(&mut req, "kw1"), 0);
    assert_eq!(cnt(&mut req, "kw2"), 0);

    Keyword::update_crate_old(req.tx().unwrap(), &krate, &["kw1".to_string()]).unwrap();
    assert_eq!(cnt(&mut req, "kw1"), 1);
    assert_eq!(cnt(&mut req, "kw2"), 0);

    Keyword::update_crate_old(req.tx().unwrap(), &krate, &["kw2".to_string()]).unwrap();
    assert_eq!(cnt(&mut req, "kw1"), 0);
    assert_eq!(cnt(&mut req, "kw2"), 1);

    Keyword::update_crate_old(req.tx().unwrap(), &krate, &[]).unwrap();
    assert_eq!(cnt(&mut req, "kw1"), 0);
    assert_eq!(cnt(&mut req, "kw2"), 0);

    Keyword::update_crate_old(req.tx().unwrap(), &krate, &["kw1".to_string(),
                                              "kw2".to_string()]).unwrap();
    assert_eq!(cnt(&mut req, "kw1"), 1);
    assert_eq!(cnt(&mut req, "kw2"), 1);

    Keyword::update_crate_old(req.tx().unwrap(), &krate, &[]).unwrap();
    assert_eq!(cnt(&mut req, "kw1"), 0);
    assert_eq!(cnt(&mut req, "kw2"), 0);

}
