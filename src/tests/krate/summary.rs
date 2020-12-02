use crate::builders::{CrateBuilder, VersionBuilder};
use crate::new_category;
use crate::util::{RequestHelper, TestApp};
use cargo_registry::schema::metadata;
use cargo_registry::views::{EncodableCategory, EncodableCrate, EncodableKeyword};
use chrono::Utc;
use diesel::{update, ExpressionMethods, RunQueryDsl};

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

#[test]
fn summary_doesnt_die() {
    let (_, anon) = TestApp::init().empty();
    anon.get::<SummaryResponse>("/api/v1/summary").good();
}

#[test]
fn summary_new_crates() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();
    app.db(|conn| {
        let now_ = Utc::now().naive_utc();
        let now_plus_two = now_ + chrono::Duration::seconds(2);

        new_category("Category 1", "cat1", "Category 1 crates")
            .create_or_update(conn)
            .unwrap();

        CrateBuilder::new("some_downloads", user.id)
            .version(VersionBuilder::new("0.1.0"))
            .description("description")
            .keyword("popular")
            .category("cat1")
            .downloads(20)
            .recent_downloads(10)
            .expect_build(conn);

        CrateBuilder::new("most_recent_downloads", user.id)
            .version(VersionBuilder::new("0.2.0"))
            .keyword("popular")
            .category("cat1")
            .downloads(5000)
            .recent_downloads(50)
            .expect_build(conn);

        CrateBuilder::new("just_updated", user.id)
            .version(VersionBuilder::new("0.1.0"))
            .version(VersionBuilder::new("0.1.2"))
            // update 'just_updated' krate. Others won't appear because updated_at == created_at.
            .updated_at(now_)
            .expect_build(conn);

        CrateBuilder::new("just_updated_patch", user.id)
            .version(VersionBuilder::new("0.1.0"))
            .version(VersionBuilder::new("0.2.0"))
            // Add a patch version be newer than the other versions, including the higher one.
            .version(VersionBuilder::new("0.1.1").created_at(now_plus_two))
            .updated_at(now_plus_two)
            .expect_build(conn);

        CrateBuilder::new("with_downloads", user.id)
            .version(VersionBuilder::new("0.3.0"))
            .keyword("popular")
            .downloads(1000)
            .expect_build(conn);

        // set total_downloads global value for `num_downloads` prop
        update(metadata::table)
            .set(metadata::total_downloads.eq(6000))
            .execute(&*conn)
            .unwrap();
    });

    let json: SummaryResponse = anon.get("/api/v1/summary").good();

    assert_eq!(json.num_crates, 5);
    assert_eq!(json.num_downloads, 6000);
    assert_eq!(json.most_downloaded[0].name, "most_recent_downloads");
    assert_eq!(json.most_downloaded[0].downloads, 5000);
    assert_eq!(json.most_downloaded[0].recent_downloads, Some(50));
    assert_eq!(
        json.most_recently_downloaded[0].name,
        "most_recent_downloads"
    );
    assert_eq!(json.most_recently_downloaded[0].recent_downloads, Some(50));
    assert_eq!(json.popular_keywords[0].keyword, "popular");
    assert_eq!(json.popular_categories[0].category, "Category 1");
    assert_eq!(json.just_updated.len(), 2);

    assert_eq!(json.just_updated[0].name, "just_updated_patch");
    assert_eq!(json.just_updated[0].max_version, "0.2.0");
    assert_eq!(json.just_updated[0].newest_version, "0.1.1");

    assert_eq!(json.just_updated[1].name, "just_updated");
    assert_eq!(json.just_updated[1].max_version, "0.1.2");
    assert_eq!(json.just_updated[1].newest_version, "0.1.2");

    assert_eq!(json.new_crates.len(), 5);
}
