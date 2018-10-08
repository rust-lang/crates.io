use models::Keyword;
use views::EncodableKeyword;
use {new_user, CrateBuilder, MockUserSession};

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
    let session = MockUserSession::anonymous();
    let json: KeywordList = session.get(url).good();
    assert_eq!(json.keywords.len(), 0);
    assert_eq!(json.meta.total, 0);

    session.db(|conn| {
        Keyword::find_or_create_all(conn, &["foo"]).unwrap();
    });

    let json: KeywordList = session.get(url).good();
    assert_eq!(json.keywords.len(), 1);
    assert_eq!(json.meta.total, 1);
    assert_eq!(json.keywords[0].keyword.as_str(), "foo");
}

#[test]
fn show() {
    let url = "/api/v1/keywords/foo";
    let session = MockUserSession::anonymous();
    session.get(url).assert_not_found();

    session.db(|conn| {
        Keyword::find_or_create_all(conn, &["foo"]).unwrap();
    });
    let json: GoodKeyword = session.get(url).good();
    assert_eq!(json.keyword.keyword.as_str(), "foo");
}

#[test]
fn uppercase() {
    let url = "/api/v1/keywords/UPPER";
    let session = MockUserSession::anonymous();
    session.get(url).assert_not_found();

    session.db(|conn| {
        Keyword::find_or_create_all(conn, &["UPPER"]).unwrap();
    });
    let json: GoodKeyword = session.get(url).good();
    assert_eq!(json.keyword.keyword.as_str(), "upper");
}

#[test]
fn update_crate() {
    let session = MockUserSession::anonymous();
    let cnt = |kw: &str| {
        let json: GoodKeyword = session.get(&format!("/api/v1/keywords/{}", kw)).good();
        json.keyword.crates_cnt as usize
    };

    let krate = session.db(|conn| {
        let u = new_user("foo").create_or_update(&conn).unwrap();
        Keyword::find_or_create_all(&conn, &["kw1", "kw2"]).unwrap();
        CrateBuilder::new("fookey", u.id).expect_build(&conn)
    });

    session.db(|conn| {
        Keyword::update_crate(&conn, &krate, &[]).unwrap();
    });
    assert_eq!(cnt("kw1"), 0);
    assert_eq!(cnt("kw2"), 0);

    session.db(|conn| {
        Keyword::update_crate(&conn, &krate, &["kw1"]).unwrap();
    });
    assert_eq!(cnt("kw1"), 1);
    assert_eq!(cnt("kw2"), 0);

    session.db(|conn| {
        Keyword::update_crate(&conn, &krate, &["kw2"]).unwrap();
    });
    assert_eq!(cnt("kw1"), 0);
    assert_eq!(cnt("kw2"), 1);

    session.db(|conn| {
        Keyword::update_crate(&conn, &krate, &[]).unwrap();
    });
    assert_eq!(cnt("kw1"), 0);
    assert_eq!(cnt("kw2"), 0);

    session.db(|conn| {
        Keyword::update_crate(&conn, &krate, &["kw1", "kw2"]).unwrap();
    });
    assert_eq!(cnt("kw1"), 1);
    assert_eq!(cnt("kw2"), 1);

    session.db(|conn| {
        Keyword::update_crate(&conn, &krate, &[]).unwrap();
    });
    assert_eq!(cnt("kw1"), 0);
    assert_eq!(cnt("kw2"), 0);
}
