use crate::util::{RequestHelper, TestApp};
use crates_io::models::Keyword;
use crates_io::views::EncodableKeyword;

#[derive(Deserialize)]
struct KeywordList {
    keywords: Vec<EncodableKeyword>,
    meta: KeywordMeta,
}

#[derive(Deserialize)]
struct KeywordMeta {
    total: i32,
}

#[tokio::test(flavor = "multi_thread")]
async fn index() {
    let url = "/api/v1/keywords";
    let (app, anon) = TestApp::init().empty();
    let json: KeywordList = anon.get(url).await.good();
    assert_eq!(json.keywords.len(), 0);
    assert_eq!(json.meta.total, 0);

    app.db(|conn| {
        Keyword::find_or_create_all(conn, &["foo"]).unwrap();
    });

    let json: KeywordList = anon.get(url).await.good();
    assert_eq!(json.keywords.len(), 1);
    assert_eq!(json.meta.total, 1);
    assert_eq!(json.keywords[0].keyword.as_str(), "foo");
}
