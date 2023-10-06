use crate::util::{RequestHelper, TestApp};

#[derive(Deserialize)]
struct UserStats {
    total_downloads: i64,
}

#[test]
fn user_total_downloads() {
    use crate::builders::CrateBuilder;
    use crate::util::{RequestHelper, TestApp};
    use diesel::{update, RunQueryDsl};

    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();
    let another_user = app.db_new_user("bar");
    let another_user = another_user.as_model();

    app.db(|conn| {
        let mut krate = CrateBuilder::new("foo_krate1", user.id).expect_build(conn);
        krate.downloads = 10;
        update(&krate).set(&krate).execute(conn).unwrap();

        let mut krate2 = CrateBuilder::new("foo_krate2", user.id).expect_build(conn);
        krate2.downloads = 20;
        update(&krate2).set(&krate2).execute(conn).unwrap();

        let mut another_krate = CrateBuilder::new("bar_krate1", another_user.id).expect_build(conn);
        another_krate.downloads = 2;
        update(&another_krate)
            .set(&another_krate)
            .execute(conn)
            .unwrap();

        let mut no_longer_my_krate = CrateBuilder::new("nacho", user.id).expect_build(conn);
        no_longer_my_krate.downloads = 5;
        update(&no_longer_my_krate)
            .set(&no_longer_my_krate)
            .execute(conn)
            .unwrap();
        no_longer_my_krate
            .owner_remove(conn, &user.gh_login)
            .unwrap();
    });

    let url = format!("/api/v1/users/{}/stats", user.id);
    let stats: UserStats = anon.get(&url).good();
    // does not include crates user never owned (2) or no longer owns (5)
    assert_eq!(stats.total_downloads, 30);
}

#[test]
fn user_total_downloads_no_crates() {
    let (_, anon, user) = TestApp::init().with_user();
    let user = user.as_model();
    let url = format!("/api/v1/users/{}/stats", user.id);

    let stats: UserStats = anon.get(&url).good();
    assert_eq!(stats.total_downloads, 0);
}
