use crate::models::Keyword;
use crate::tests::builders::CrateBuilder;
use crate::tests::util::{RequestHelper, TestApp};
use crate::views::EncodableKeyword;

#[derive(Deserialize)]
struct GoodKeyword {
    keyword: EncodableKeyword,
}

#[tokio::test(flavor = "multi_thread")]
async fn show() {
    let url = "/api/v1/keywords/foo";
    let (app, anon) = TestApp::init().empty();
    let mut conn = app.db_conn();

    anon.get(url).await.assert_not_found();

    Keyword::find_or_create_all(&mut conn, &["foo"]).unwrap();

    let json: GoodKeyword = anon.get(url).await.good();
    assert_eq!(json.keyword.keyword.as_str(), "foo");
}

#[tokio::test(flavor = "multi_thread")]
async fn uppercase() {
    let url = "/api/v1/keywords/UPPER";
    let (app, anon) = TestApp::init().empty();
    let mut conn = app.db_conn();

    anon.get(url).await.assert_not_found();

    Keyword::find_or_create_all(&mut conn, &["UPPER"]).unwrap();

    let json: GoodKeyword = anon.get(url).await.good();
    assert_eq!(json.keyword.keyword.as_str(), "upper");
}

#[tokio::test(flavor = "multi_thread")]
async fn update_crate() {
    let (app, anon, user) = TestApp::init().with_user();
    let mut conn = app.db_conn();
    let user = user.as_model();

    async fn cnt(kw: &str, client: &impl RequestHelper) -> usize {
        let json: GoodKeyword = client.get(&format!("/api/v1/keywords/{kw}")).await.good();
        json.keyword.crates_cnt as usize
    }

    Keyword::find_or_create_all(&mut conn, &["kw1", "kw2"]).unwrap();
    let krate = CrateBuilder::new("fookey", user.id).expect_build(&mut conn);

    Keyword::update_crate(&mut conn, krate.id, &[]).unwrap();
    assert_eq!(cnt("kw1", &anon).await, 0);
    assert_eq!(cnt("kw2", &anon).await, 0);

    Keyword::update_crate(&mut conn, krate.id, &["kw1"]).unwrap();
    assert_eq!(cnt("kw1", &anon).await, 1);
    assert_eq!(cnt("kw2", &anon).await, 0);

    Keyword::update_crate(&mut conn, krate.id, &["kw2"]).unwrap();
    assert_eq!(cnt("kw1", &anon).await, 0);
    assert_eq!(cnt("kw2", &anon).await, 1);

    Keyword::update_crate(&mut conn, krate.id, &[]).unwrap();
    assert_eq!(cnt("kw1", &anon).await, 0);
    assert_eq!(cnt("kw2", &anon).await, 0);

    Keyword::update_crate(&mut conn, krate.id, &["kw1", "kw2"]).unwrap();
    assert_eq!(cnt("kw1", &anon).await, 1);
    assert_eq!(cnt("kw2", &anon).await, 1);

    Keyword::update_crate(&mut conn, krate.id, &[]).unwrap();
    assert_eq!(cnt("kw1", &anon).await, 0);
    assert_eq!(cnt("kw2", &anon).await, 0);
}
