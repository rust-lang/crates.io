use conduit::{Handler, Method};
use conduit_test::MockRequest;

use models::Keyword;
use views::EncodableKeyword;
use {app, new_user, req, CrateBuilder, MockUserSession};

#[derive(Deserialize)]
struct KeywordList {
    keywords: Vec<EncodableKeyword>,
    meta: KeywordMeta,
}
#[derive(Deserialize)]
struct KeywordMeta {
    total: i32,
}
#[derive(Deserialize)]
struct GoodKeyword {
    keyword: EncodableKeyword,
}

#[test]
fn index() {
    let url = "/api/v1/keywords";
    let mut session = MockUserSession::anonymous();
    let json: KeywordList = session.get(url).good();
    assert_eq!(json.keywords.len(), 0);
    assert_eq!(json.meta.total, 0);

    session.db(|conn| {
        Keyword::find_or_create_all(conn, &["foo"]).unwrap();
    });

    let json: KeywordList = session.get(url).good();
    assert_eq!(json.keywords.len(), 1);
    assert_eq!(json.meta.total, 1);
    assert_eq!(json.keywords[0].keyword, "foo".to_string());
}

#[test]
fn show() {
    let url = "/api/v1/keywords/foo";
    let mut session = MockUserSession::anonymous();
    session.get::<()>(url).assert_status(404);

    session.db(|conn| {
        Keyword::find_or_create_all(conn, &["foo"]).unwrap();
    });
    let json: GoodKeyword = session.get(url).good();
    assert_eq!(json.keyword.keyword, "foo".to_string());
}

#[test]
fn uppercase() {
    let url = "/api/v1/keywords/UPPER";
    let mut session = MockUserSession::anonymous();
    session.get::<()>(url).assert_status(404);

    session.db(|conn| {
        Keyword::find_or_create_all(conn, &["UPPER"]).unwrap();
    });
    let json: GoodKeyword = session.get(url).good();
    assert_eq!(json.keyword.keyword, "upper".to_string());
}

#[test]
fn update_crate() {
    let (_b, app, middle) = app();
    let mut req = req(Method::Get, "/api/v1/keywords/foo");
    let cnt = |req: &mut MockRequest, kw: &str| {
        req.with_path(&format!("/api/v1/keywords/{}", kw));
        let mut response = ok_resp!(middle.call(req));
        ::json::<GoodKeyword>(&mut response).keyword.crates_cnt as usize
    };

    let krate = {
        let conn = app.diesel_database.get().unwrap();
        let u = new_user("foo").create_or_update(&conn).unwrap();
        Keyword::find_or_create_all(&conn, &["kw1", "kw2"]).unwrap();
        CrateBuilder::new("fookey", u.id).expect_build(&conn)
    };

    {
        let conn = app.diesel_database.get().unwrap();
        Keyword::update_crate(&conn, &krate, &[]).unwrap();
    }
    assert_eq!(cnt(&mut req, "kw1"), 0);
    assert_eq!(cnt(&mut req, "kw2"), 0);

    {
        let conn = app.diesel_database.get().unwrap();
        Keyword::update_crate(&conn, &krate, &["kw1"]).unwrap();
    }
    assert_eq!(cnt(&mut req, "kw1"), 1);
    assert_eq!(cnt(&mut req, "kw2"), 0);

    {
        let conn = app.diesel_database.get().unwrap();
        Keyword::update_crate(&conn, &krate, &["kw2"]).unwrap();
    }
    assert_eq!(cnt(&mut req, "kw1"), 0);
    assert_eq!(cnt(&mut req, "kw2"), 1);

    {
        let conn = app.diesel_database.get().unwrap();
        Keyword::update_crate(&conn, &krate, &[]).unwrap();
    }
    assert_eq!(cnt(&mut req, "kw1"), 0);
    assert_eq!(cnt(&mut req, "kw2"), 0);

    {
        let conn = app.diesel_database.get().unwrap();
        Keyword::update_crate(&conn, &krate, &["kw1", "kw2"]).unwrap();
    }
    assert_eq!(cnt(&mut req, "kw1"), 1);
    assert_eq!(cnt(&mut req, "kw2"), 1);

    {
        let conn = app.diesel_database.get().unwrap();
        Keyword::update_crate(&conn, &krate, &[]).unwrap();
    }
    assert_eq!(cnt(&mut req, "kw1"), 0);
    assert_eq!(cnt(&mut req, "kw2"), 0);
}
