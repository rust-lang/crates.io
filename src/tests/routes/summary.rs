use crate::schema::metadata;
use crate::tests::builders::{CrateBuilder, VersionBuilder};
use crate::tests::new_category;
use crate::tests::util::{RequestHelper, TestApp};
use crate::views::{EncodableCategory, EncodableCrate, EncodableKeyword};
use chrono::Utc;
use crates_io_database::schema::categories;
use diesel::{insert_into, update, ExpressionMethods};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};

#[derive(Deserialize)]
struct SummaryResponse {
    num_downloads: i64,
    num_crates: i64,
    new_crates: Vec<EncodableCrate>,
    most_downloaded: Vec<EncodableCrate>,
    most_recently_downloaded: Vec<EncodableCrate>,
    just_updated: Vec<EncodableCrate>,
    popular_keywords: Vec<EncodableKeyword>,
    popular_categories: Vec<EncodableCategory>,
}

#[tokio::test(flavor = "multi_thread")]
async fn summary_doesnt_die() {
    let (_, anon) = TestApp::init().empty().await;
    anon.get::<SummaryResponse>("/api/v1/summary").await.good();
}

#[tokio::test(flavor = "multi_thread")]
async fn summary_new_crates() {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    conn.transaction(|conn| {
        async move {
            let now_ = Utc::now().naive_utc();
            let now_plus_two = now_ + chrono::Duration::seconds(2);

            insert_into(categories::table)
                .values(new_category("Category 1", "cat1", "Category 1 crates"))
                .execute(conn)
                .await
                .unwrap();

            CrateBuilder::new("some_downloads", user.id)
                .version(VersionBuilder::new("0.1.0"))
                .description("description")
                .keyword("popular")
                .category("cat1")
                .downloads(20)
                .recent_downloads(10)
                .expect_build(conn)
                .await;

            CrateBuilder::new("most_recent_downloads", user.id)
                .version(VersionBuilder::new("0.2.0"))
                .version(VersionBuilder::new("0.2.1").yanked(true))
                .keyword("popular")
                .category("cat1")
                .downloads(5000)
                .recent_downloads(50)
                .expect_build(conn)
                .await;

            CrateBuilder::new("just_updated", user.id)
                .version(VersionBuilder::new("0.1.0"))
                .version(VersionBuilder::new("0.1.1").yanked(true))
                .version(VersionBuilder::new("0.1.2"))
                // update 'just_updated' krate. Others won't appear because updated_at == created_at.
                .updated_at(now_)
                .expect_build(conn)
                .await;

            CrateBuilder::new("just_updated_patch", user.id)
                .version(VersionBuilder::new("0.1.0"))
                .version(VersionBuilder::new("0.2.0"))
                .version(VersionBuilder::new("0.2.1").yanked(true))
                // Add a patch version be newer than the other versions, including the higher one.
                .version(VersionBuilder::new("0.1.1").created_at(now_plus_two))
                .updated_at(now_plus_two)
                .expect_build(conn)
                .await;

            CrateBuilder::new("with_downloads", user.id)
                .version(VersionBuilder::new("0.3.0"))
                .version(VersionBuilder::new("0.3.1").yanked(true))
                .keyword("popular")
                .downloads(1000)
                .expect_build(conn)
                .await;

            // set total_downloads global value for `num_downloads` prop
            update(metadata::table)
                .set(metadata::total_downloads.eq(6000))
                .execute(conn)
                .await
                .unwrap();

            Ok::<_, anyhow::Error>(())
        }
        .scope_boxed()
    })
    .await
    .unwrap();

    let json: SummaryResponse = anon.get("/api/v1/summary").await.good();

    assert_eq!(json.num_crates, 5);
    assert_eq!(json.num_downloads, 6000);
    assert_eq!(json.most_downloaded[0].name, "most_recent_downloads");
    assert_eq!(
        json.most_downloaded[0].default_version,
        Some("0.2.0".into())
    );
    assert!(!json.most_downloaded[0].yanked);
    assert_eq!(json.most_downloaded[0].downloads, 5000);
    assert_eq!(json.most_downloaded[0].recent_downloads, Some(50));
    assert_eq!(
        json.most_recently_downloaded[0].name,
        "most_recent_downloads"
    );
    assert_eq!(
        json.most_recently_downloaded[0].default_version,
        Some("0.2.0".into())
    );
    assert!(!json.most_recently_downloaded[0].yanked);
    assert_eq!(json.most_recently_downloaded[0].recent_downloads, Some(50));
    assert_eq!(json.popular_keywords[0].keyword, "popular");
    assert_eq!(json.popular_categories[0].category, "Category 1");
    assert_eq!(json.just_updated.len(), 2);

    assert_eq!(json.just_updated[0].name, "just_updated_patch");
    assert_eq!(json.just_updated[0].default_version, Some("0.2.0".into()));
    assert!(!json.just_updated[0].yanked);
    assert_eq!(json.just_updated[0].max_version, "0.2.0");
    assert_eq!(json.just_updated[0].newest_version, "0.1.1");

    assert_eq!(json.just_updated[1].name, "just_updated");
    assert_eq!(json.just_updated[1].default_version, Some("0.1.2".into()));
    assert!(!json.just_updated[1].yanked);
    assert_eq!(json.just_updated[1].max_version, "0.1.2");
    assert_eq!(json.just_updated[1].newest_version, "0.1.2");

    assert_eq!(json.new_crates[0].name, "with_downloads");
    assert_eq!(json.new_crates[0].default_version, Some("0.3.0".into()));
    assert!(!json.new_crates[0].yanked);
    assert_eq!(json.new_crates.len(), 5);
}

#[tokio::test(flavor = "multi_thread")]
async fn excluded_crate_id() {
    let (app, anon, user) = TestApp::init()
        .with_config(|config| {
            config.excluded_crate_names = vec![
                "most_recent_downloads".into(),
                // make sure no error occurs with a crate name that doesn't exist and that the name
                // matches are exact, not substrings
                "downloads".into(),
            ];
        })
        .with_user()
        .await;

    let mut conn = app.db_conn().await;
    let user = user.as_model();

    CrateBuilder::new("some_downloads", user.id)
        .version(VersionBuilder::new("0.1.0"))
        .version(VersionBuilder::new("0.2.0").yanked(true))
        .description("description")
        .keyword("popular")
        .category("cat1")
        .downloads(20)
        .recent_downloads(10)
        .expect_build(&mut conn)
        .await;

    CrateBuilder::new("most_recent_downloads", user.id)
        .version(VersionBuilder::new("0.2.0"))
        .keyword("popular")
        .category("cat1")
        .downloads(5000)
        .recent_downloads(50)
        .expect_build(&mut conn)
        .await;

    let json: SummaryResponse = anon.get("/api/v1/summary").await.good();

    assert_eq!(json.most_downloaded.len(), 1);
    assert_eq!(json.most_downloaded[0].name, "some_downloads");
    assert_eq!(
        json.most_downloaded[0].default_version,
        Some("0.1.0".into())
    );
    assert!(!json.most_downloaded[0].yanked);
    assert_eq!(json.most_downloaded[0].downloads, 20);

    assert_eq!(json.most_recently_downloaded.len(), 1);
    assert_eq!(json.most_recently_downloaded[0].name, "some_downloads");
    assert_eq!(
        json.most_recently_downloaded[0].default_version,
        Some("0.1.0".into())
    );
    assert!(!json.most_recently_downloaded[0].yanked);
    assert_eq!(json.most_recently_downloaded[0].recent_downloads, Some(10));
}

#[tokio::test(flavor = "multi_thread")]
async fn all_yanked() {
    let (app, anon, user) = TestApp::init()
        .with_config(|config| {
            config.excluded_crate_names = vec![
                "most_recent_downloads".into(),
                // make sure no error occurs with a crate name that doesn't exist and that the name
                // matches are exact, not substrings
                "downloads".into(),
            ];
        })
        .with_user()
        .await;

    let mut conn = app.db_conn().await;
    let user = user.as_model();

    CrateBuilder::new("some_downloads", user.id)
        .version(VersionBuilder::new("0.1.0").yanked(true))
        .version(VersionBuilder::new("0.2.0").yanked(true))
        .description("description")
        .keyword("popular")
        .category("cat1")
        .downloads(20)
        .recent_downloads(10)
        .expect_build(&mut conn)
        .await;

    CrateBuilder::new("most_recent_downloads", user.id)
        .version(VersionBuilder::new("0.2.0"))
        .keyword("popular")
        .category("cat1")
        .downloads(5000)
        .recent_downloads(50)
        .expect_build(&mut conn)
        .await;

    let json: SummaryResponse = anon.get("/api/v1/summary").await.good();

    assert_eq!(json.most_downloaded.len(), 1);
    assert_eq!(json.most_downloaded[0].name, "some_downloads");
    assert_eq!(
        json.most_downloaded[0].default_version,
        Some("0.2.0".into())
    );
    assert!(json.most_downloaded[0].yanked);
    assert_eq!(json.most_downloaded[0].downloads, 20);

    assert_eq!(json.most_recently_downloaded.len(), 1);
    assert_eq!(json.most_recently_downloaded[0].name, "some_downloads");
    assert_eq!(
        json.most_recently_downloaded[0].default_version,
        Some("0.2.0".into())
    );
    assert!(json.most_recently_downloaded[0].yanked);
    assert_eq!(json.most_recently_downloaded[0].recent_downloads, Some(10));
}
