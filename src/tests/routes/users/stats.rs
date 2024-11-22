use crate::tests::util::{RequestHelper, TestApp};

#[derive(Deserialize)]
struct UserStats {
    total_downloads: i64,
}

#[tokio::test(flavor = "multi_thread")]
async fn user_total_downloads() -> anyhow::Result<()> {
    use crate::schema::crate_downloads;
    use crate::tests::builders::CrateBuilder;
    use crate::tests::util::{RequestHelper, TestApp};
    use diesel::prelude::*;
    use diesel::{update, QueryDsl};
    use diesel_async::RunQueryDsl;

    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();
    let another_user = app.db_new_user("bar").await;
    let another_user = another_user.as_model();

    let krate = CrateBuilder::new("foo_krate1", user.id)
        .expect_build(&mut conn)
        .await;
    update(crate_downloads::table.filter(crate_downloads::crate_id.eq(krate.id)))
        .set(crate_downloads::downloads.eq(10))
        .execute(&mut conn)
        .await?;

    let krate2 = CrateBuilder::new("foo_krate2", user.id)
        .expect_build(&mut conn)
        .await;
    update(crate_downloads::table.filter(crate_downloads::crate_id.eq(krate2.id)))
        .set(crate_downloads::downloads.eq(20))
        .execute(&mut conn)
        .await?;

    let another_krate = CrateBuilder::new("bar_krate1", another_user.id)
        .expect_build(&mut conn)
        .await;
    update(crate_downloads::table.filter(crate_downloads::crate_id.eq(another_krate.id)))
        .set(crate_downloads::downloads.eq(2))
        .execute(&mut conn)
        .await?;

    let no_longer_my_krate = CrateBuilder::new("nacho", user.id)
        .expect_build(&mut conn)
        .await;
    update(crate_downloads::table.filter(crate_downloads::crate_id.eq(no_longer_my_krate.id)))
        .set(crate_downloads::downloads.eq(5))
        .execute(&mut conn)
        .await?;
    no_longer_my_krate
        .owner_remove(&mut conn, &user.gh_login)
        .await
        .unwrap();

    let url = format!("/api/v1/users/{}/stats", user.id);
    let stats: UserStats = anon.get(&url).await.good();
    // does not include crates user never owned (2) or no longer owns (5)
    assert_eq!(stats.total_downloads, 30);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn user_total_downloads_no_crates() {
    let (_, anon, user) = TestApp::init().with_user().await;
    let user = user.as_model();
    let url = format!("/api/v1/users/{}/stats", user.id);

    let stats: UserStats = anon.get(&url).await.good();
    assert_eq!(stats.total_downloads, 0);
}
