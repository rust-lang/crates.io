use models::Keyword;
use views::EncodableKeyword;
use {new_user, CrateBuilder, RequestHelper, TestApp};

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
    let (app, anon) = TestApp::empty();
    let json: KeywordList = anon.get(url).good();
    assert_eq!(json.keywords.len(), 0);
    assert_eq!(json.meta.total, 0);

    app.db(|conn| {
        Keyword::find_or_create_all(conn, &["foo"]).unwrap();
    });

    let json: KeywordList = anon.get(url).good();
    assert_eq!(json.keywords.len(), 1);
    assert_eq!(json.meta.total, 1);
    assert_eq!(json.keywords[0].keyword.as_str(), "foo");
}

#[test]
fn show() {
    let url = "/api/v1/keywords/foo";
    let (app, anon) = TestApp::empty();
    anon.get(url).assert_not_found();

    app.db(|conn| {
        Keyword::find_or_create_all(conn, &["foo"]).unwrap();
    });
    let json: GoodKeyword = anon.get(url).good();
    assert_eq!(json.keyword.keyword.as_str(), "foo");
}

#[test]
fn uppercase() {
    let url = "/api/v1/keywords/UPPER";
    let (app, anon) = TestApp::empty();
    anon.get(url).assert_not_found();

    app.db(|conn| {
        Keyword::find_or_create_all(conn, &["UPPER"]).unwrap();
    });
    let json: GoodKeyword = anon.get(url).good();
    assert_eq!(json.keyword.keyword.as_str(), "upper");
}

#[test]
fn update_crate() {
    let (app, anon) = TestApp::empty();
    let cnt = |kw: &str| {
        let json: GoodKeyword = anon.get(&format!("/api/v1/keywords/{}", kw)).good();
        json.keyword.crates_cnt as usize
    };

    let krate = app.db(|conn| {
        let u = new_user("foo").create_or_update(&conn).unwrap();
        Keyword::find_or_create_all(&conn, &["kw1", "kw2"]).unwrap();
        CrateBuilder::new("fookey", u.id).expect_build(&conn)
    });

    app.db(|conn| {
        Keyword::update_crate(&conn, &krate, &[]).unwrap();
    });
    assert_eq!(cnt("kw1"), 0);
    assert_eq!(cnt("kw2"), 0);

    app.db(|conn| {
        Keyword::update_crate(&conn, &krate, &["kw1"]).unwrap();
    });
    assert_eq!(cnt("kw1"), 1);
    assert_eq!(cnt("kw2"), 0);

    app.db(|conn| {
        Keyword::update_crate(&conn, &krate, &["kw2"]).unwrap();
    });
    assert_eq!(cnt("kw1"), 0);
    assert_eq!(cnt("kw2"), 1);

    app.db(|conn| {
        Keyword::update_crate(&conn, &krate, &[]).unwrap();
    });
    assert_eq!(cnt("kw1"), 0);
    assert_eq!(cnt("kw2"), 0);

    app.db(|conn| {
        Keyword::update_crate(&conn, &krate, &["kw1", "kw2"]).unwrap();
    });
    assert_eq!(cnt("kw1"), 1);
    assert_eq!(cnt("kw2"), 1);

    app.db(|conn| {
        Keyword::update_crate(&conn, &krate, &[]).unwrap();
    });
    assert_eq!(cnt("kw1"), 0);
    assert_eq!(cnt("kw2"), 0);
}
