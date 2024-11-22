use crate::models::Keyword;
use crate::tests::builders::CrateBuilder;
use crate::tests::util::{RequestHelper, TestApp};
use crate::views::EncodableKeyword;

#[derive(Deserialize)]
struct GoodKeyword {
    keyword: EncodableKeyword,
}

#[tokio::test(flavor = "multi_thread")]
async fn show() -> anyhow::Result<()> {
    let url = "/api/v1/keywords/foo";
    let (app, anon) = TestApp::init().empty().await;
    let mut conn = app.db_conn().await;

    anon.get(url).await.assert_not_found();

    Keyword::find_or_create_all(&mut conn, &["foo"]).await?;

    let json: GoodKeyword = anon.get(url).await.good();
    assert_eq!(json.keyword.keyword.as_str(), "foo");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn uppercase() -> anyhow::Result<()> {
    let url = "/api/v1/keywords/UPPER";
    let (app, anon) = TestApp::init().empty().await;
    let mut conn = app.db_conn().await;

    anon.get(url).await.assert_not_found();

    Keyword::find_or_create_all(&mut conn, &["UPPER"]).await?;

    let json: GoodKeyword = anon.get(url).await.good();
    assert_eq!(json.keyword.keyword.as_str(), "upper");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn update_crate() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    async fn cnt(kw: &str, client: &impl RequestHelper) -> usize {
        let json: GoodKeyword = client.get(&format!("/api/v1/keywords/{kw}")).await.good();
        json.keyword.crates_cnt as usize
    }

    Keyword::find_or_create_all(&mut conn, &["kw1", "kw2"]).await?;
    let krate = CrateBuilder::new("fookey", user.id)
        .expect_build(&mut conn)
        .await;

    Keyword::update_crate(&mut conn, krate.id, &[]).await?;
    assert_eq!(cnt("kw1", &anon).await, 0);
    assert_eq!(cnt("kw2", &anon).await, 0);

    Keyword::update_crate(&mut conn, krate.id, &["kw1"]).await?;
    assert_eq!(cnt("kw1", &anon).await, 1);
    assert_eq!(cnt("kw2", &anon).await, 0);

    Keyword::update_crate(&mut conn, krate.id, &["kw2"]).await?;
    assert_eq!(cnt("kw1", &anon).await, 0);
    assert_eq!(cnt("kw2", &anon).await, 1);

    Keyword::update_crate(&mut conn, krate.id, &[]).await?;
    assert_eq!(cnt("kw1", &anon).await, 0);
    assert_eq!(cnt("kw2", &anon).await, 0);

    Keyword::update_crate(&mut conn, krate.id, &["kw1", "kw2"]).await?;
    assert_eq!(cnt("kw1", &anon).await, 1);
    assert_eq!(cnt("kw2", &anon).await, 1);

    Keyword::update_crate(&mut conn, krate.id, &[]).await?;
    assert_eq!(cnt("kw1", &anon).await, 0);
    assert_eq!(cnt("kw2", &anon).await, 0);

    Ok(())
}
