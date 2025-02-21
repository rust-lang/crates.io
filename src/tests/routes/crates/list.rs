use crate::models::Category;
use crate::schema::crates;
use crate::tests::builders::{CrateBuilder, VersionBuilder};
use crate::tests::util::{RequestHelper, TestApp};
use crate::tests::{new_category, new_user};
use crates_io_database::schema::categories;
use diesel::sql_types::Timestamptz;
use diesel::{dsl::*, prelude::*, update};
use diesel_async::RunQueryDsl;
use googletest::prelude::*;
use http::StatusCode;
use insta::{assert_json_snapshot, assert_snapshot};
use regex::Regex;
use std::sync::LazyLock;

#[tokio::test(flavor = "multi_thread")]
async fn index() -> anyhow::Result<()> {
    let (app, anon) = TestApp::init().empty().await;
    let mut conn = app.db_conn().await;

    for json in search_both(&anon, "").await {
        assert_eq!(json.crates.len(), 0);
        assert_eq!(json.meta.total, 0);
    }

    let user_id = new_user("foo").insert(&mut conn).await?.id;

    let krate = CrateBuilder::new("fooindex", user_id)
        .expect_build(&mut conn)
        .await;

    for json in search_both(&anon, "").await {
        assert_eq!(json.crates.len(), 1);
        assert_eq!(json.meta.total, 1);
        assert_eq!(json.crates[0].name, krate.name);
        assert_eq!(json.crates[0].id, krate.name);
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[allow(clippy::cognitive_complexity)]
async fn index_queries() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    let krate = CrateBuilder::new("foo_index_queries", user.id)
        .readme("readme")
        .description("description")
        .keyword("kw1")
        .expect_build(&mut conn)
        .await;

    let krate2 = CrateBuilder::new("BAR_INDEX_QUERIES", user.id)
        .keyword("KW1")
        .expect_build(&mut conn)
        .await;

    CrateBuilder::new("foo", user.id)
        .keyword("kw3")
        .expect_build(&mut conn)
        .await;

    CrateBuilder::new("two-keywords", user.id)
        .keyword("kw1")
        .keyword("kw3")
        .expect_build(&mut conn)
        .await;

    for json in search_both(&anon, "q=baz").await {
        assert_eq!(json.crates.len(), 0);
        assert_eq!(json.meta.total, 0);
    }

    // All of these fields should be indexed/searched by the queries
    for json in search_both(&anon, "q=foo").await {
        assert_eq!(json.crates.len(), 2);
        assert_eq!(json.meta.total, 2);
    }

    for json in search_both(&anon, "q=kw1").await {
        assert_eq!(json.crates.len(), 3);
        assert_eq!(json.meta.total, 3);
    }

    for json in search_both(&anon, "q=readme").await {
        assert_eq!(json.crates.len(), 1);
        assert_eq!(json.meta.total, 1);
    }

    for json in search_both(&anon, "q=description").await {
        assert_eq!(json.crates.len(), 1);
        assert_eq!(json.meta.total, 1);
    }

    // Query containing a space
    for json in search_both(&anon, "q=foo%20kw3").await {
        assert_eq!(json.crates.len(), 1);
        assert_eq!(json.meta.total, 1);
    }

    for json in search_both_by_user_id(&anon, user.id).await {
        assert_eq!(json.crates.len(), 4);
        assert_eq!(json.meta.total, 4);
    }

    for json in search_both_by_user_id(&anon, 0).await {
        assert_eq!(json.crates.len(), 0);
        assert_eq!(json.meta.total, 0);
    }

    for json in search_both(&anon, "letter=F").await {
        assert_eq!(json.crates.len(), 2);
        assert_eq!(json.meta.total, 2);
    }

    for json in search_both(&anon, "letter=B").await {
        assert_eq!(json.crates.len(), 1);
        assert_eq!(json.meta.total, 1);
    }

    for json in search_both(&anon, "letter=b").await {
        assert_eq!(json.crates.len(), 1);
        assert_eq!(json.meta.total, 1);
    }

    for json in search_both(&anon, "letter=c").await {
        assert_eq!(json.crates.len(), 0);
        assert_eq!(json.meta.total, 0);
    }

    for json in search_both(&anon, "keyword=kw1").await {
        assert_eq!(json.crates.len(), 3);
        assert_eq!(json.meta.total, 3);
    }

    for json in search_both(&anon, "keyword=KW1").await {
        assert_eq!(json.crates.len(), 3);
        assert_eq!(json.meta.total, 3);
    }

    for json in search_both(&anon, "keyword=kw2").await {
        assert_eq!(json.crates.len(), 0);
        assert_eq!(json.meta.total, 0);
    }

    for json in search_both(&anon, "all_keywords=kw1%20kw3").await {
        assert_eq!(json.crates.len(), 1);
        assert_eq!(json.meta.total, 1);
    }

    for json in search_both(&anon, "q=foo&keyword=kw1").await {
        assert_eq!(json.crates.len(), 1);
        assert_eq!(json.meta.total, 1);
    }

    for json in search_both(&anon, "q=foo2&keyword=kw1").await {
        assert_eq!(json.crates.len(), 0);
        assert_eq!(json.meta.total, 0);
    }

    let cats = vec![
        new_category("Category 1", "cat1", "Category 1 crates"),
        new_category("Category 1::Ba'r", "cat1::bar", "Ba'r crates"),
    ];

    insert_into(categories::table)
        .values(cats)
        .execute(&mut conn)
        .await?;

    Category::update_crate(&mut conn, krate.id, &["cat1"]).await?;
    Category::update_crate(&mut conn, krate2.id, &["cat1::bar"]).await?;

    for cl in search_both(&anon, "category=cat1").await {
        assert_eq!(cl.crates.len(), 2);
        assert_eq!(cl.meta.total, 2);
    }

    for cl in search_both(&anon, "category=cat1::bar").await {
        assert_eq!(cl.crates.len(), 1);
        assert_eq!(cl.meta.total, 1);
    }

    for cl in search_both(&anon, "keyword=cat2").await {
        assert_eq!(cl.crates.len(), 0);
        assert_eq!(cl.meta.total, 0);
    }

    for cl in search_both(&anon, "q=readme&category=cat1").await {
        assert_eq!(cl.crates.len(), 1);
        assert_eq!(cl.meta.total, 1);
    }

    for cl in search_both(&anon, "keyword=kw1&category=cat1").await {
        assert_eq!(cl.crates.len(), 2);
        assert_eq!(cl.meta.total, 2);
    }

    for cl in search_both(&anon, "keyword=kw3&category=cat1").await {
        assert_eq!(cl.crates.len(), 0);
        assert_eq!(cl.meta.total, 0);
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn search_includes_crates_where_name_is_stopword() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    CrateBuilder::new("which", user.id)
        .expect_build(&mut conn)
        .await;
    CrateBuilder::new("should_be_excluded", user.id)
        .readme("crate which does things")
        .expect_build(&mut conn)
        .await;

    for json in search_both(&anon, "q=which").await {
        assert_eq!(json.crates.len(), 1);
        assert_eq!(json.meta.total, 1);
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn exact_match_first_on_queries() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    CrateBuilder::new("foo_exact", user.id)
        .description("bar_exact baz_exact")
        .expect_build(&mut conn)
        .await;

    CrateBuilder::new("bar-exact", user.id)
        .description("foo_exact baz_exact foo-exact baz_exact")
        .expect_build(&mut conn)
        .await;

    CrateBuilder::new("baz_exact", user.id)
        .description("foo-exact bar_exact foo-exact bar_exact foo_exact bar_exact")
        .expect_build(&mut conn)
        .await;

    CrateBuilder::new("other_exact", user.id)
        .description("other_exact")
        .expect_build(&mut conn)
        .await;

    for json in search_both(&anon, "q=foo-exact").await {
        assert_eq!(json.meta.total, 3);
        assert_eq!(json.crates[0].name, "foo_exact");
        assert_eq!(json.crates[1].name, "baz_exact");
        assert_eq!(json.crates[2].name, "bar-exact");
    }

    for json in search_both(&anon, "q=bar_exact").await {
        assert_eq!(json.meta.total, 3);
        assert_eq!(json.crates[0].name, "bar-exact");
        assert_eq!(json.crates[1].name, "baz_exact");
        assert_eq!(json.crates[2].name, "foo_exact");
    }

    for json in search_both(&anon, "q=baz_exact").await {
        assert_eq!(json.meta.total, 3);
        assert_eq!(json.crates[0].name, "baz_exact");
        assert_eq!(json.crates[1].name, "bar-exact");
        assert_eq!(json.crates[2].name, "foo_exact");
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[allow(clippy::cognitive_complexity)]
async fn index_sorting() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    // To test that the unique ordering of seed-based pagination is correct, we need to
    // set some columns to the same value.

    let krate1 = CrateBuilder::new("foo_sort", user.id)
        .description("bar_sort baz_sort const")
        .downloads(50)
        .recent_downloads(50)
        .expect_build(&mut conn)
        .await;

    let krate2 = CrateBuilder::new("bar_sort", user.id)
        .description("foo_sort baz_sort foo_sort baz_sort const")
        .downloads(3333)
        .recent_downloads(0)
        .expect_build(&mut conn)
        .await;

    let krate3 = CrateBuilder::new("baz_sort", user.id)
        .description("foo_sort bar_sort foo_sort bar_sort bar_sort const")
        .downloads(100_000)
        .recent_downloads(50)
        .expect_build(&mut conn)
        .await;

    let krate4 = CrateBuilder::new("other_sort", user.id)
        .description("other_sort const")
        .downloads(100_000)
        .expect_build(&mut conn)
        .await;

    // Set the created at column for each crate
    update(&krate1)
        .set(crates::created_at.eq(now.into_sql::<Timestamptz>() - 4.weeks()))
        .execute(&mut conn)
        .await?;
    update(&krate2)
        .set(crates::created_at.eq(now.into_sql::<Timestamptz>() - 1.weeks()))
        .execute(&mut conn)
        .await?;
    update(crates::table.filter(crates::id.eq_any(vec![krate3.id, krate4.id])))
        .set(crates::created_at.eq(now.into_sql::<Timestamptz>() - 3.weeks()))
        .execute(&mut conn)
        .await?;

    // Set the updated at column for each crate
    update(&krate1)
        .set(crates::updated_at.eq(now.into_sql::<Timestamptz>() - 3.weeks()))
        .execute(&mut conn)
        .await?;
    update(crates::table.filter(crates::id.eq_any(vec![krate2.id, krate3.id])))
        .set(crates::updated_at.eq(now.into_sql::<Timestamptz>() - 5.days()))
        .execute(&mut conn)
        .await?;
    update(&krate4)
        .set(crates::updated_at.eq(now))
        .execute(&mut conn)
        .await?;

    // Sort by downloads
    for json in search_both(&anon, "sort=downloads").await {
        assert_eq!(json.meta.total, 4);
        assert_eq!(json.crates[0].name, "other_sort");
        assert_eq!(json.crates[1].name, "baz_sort");
        assert_eq!(json.crates[2].name, "bar_sort");
        assert_eq!(json.crates[3].name, "foo_sort");
    }
    let (resp, calls) = page_with_seek(&anon, "sort=downloads").await;
    assert_eq!(resp[0].crates[0].name, "other_sort");
    assert_eq!(resp[1].crates[0].name, "baz_sort");
    assert_eq!(resp[2].crates[0].name, "bar_sort");
    assert_eq!(resp[3].crates[0].name, "foo_sort");
    assert_eq!(resp[3].meta.total, 4);
    assert_eq!(calls, 5);

    // Sort by recent-downloads
    for json in search_both(&anon, "sort=recent-downloads").await {
        assert_eq!(json.meta.total, 4);
        assert_eq!(json.crates[0].name, "baz_sort");
        assert_eq!(json.crates[1].name, "foo_sort");
        assert_eq!(json.crates[2].name, "bar_sort");
        assert_eq!(json.crates[3].name, "other_sort");
    }
    let (resp, calls) = page_with_seek(&anon, "sort=recent-downloads").await;
    assert_eq!(resp[0].crates[0].name, "baz_sort");
    assert_eq!(resp[1].crates[0].name, "foo_sort");
    assert_eq!(resp[2].crates[0].name, "bar_sort");
    assert_eq!(resp[3].crates[0].name, "other_sort");
    assert_eq!(resp[3].meta.total, 4);
    assert_eq!(calls, 5);

    // Sort by recent-updates
    for json in search_both(&anon, "sort=recent-updates").await {
        assert_eq!(json.meta.total, 4);
        assert_eq!(json.crates[0].name, "other_sort");
        assert_eq!(json.crates[1].name, "baz_sort");
        assert_eq!(json.crates[2].name, "bar_sort");
        assert_eq!(json.crates[3].name, "foo_sort");
    }
    let (resp, calls) = page_with_seek(&anon, "sort=recent-updates").await;
    assert_eq!(resp[0].crates[0].name, "other_sort");
    assert_eq!(resp[1].crates[0].name, "baz_sort");
    assert_eq!(resp[2].crates[0].name, "bar_sort");
    assert_eq!(resp[3].crates[0].name, "foo_sort");
    assert_eq!(resp[3].meta.total, 4);
    assert_eq!(calls, 5);

    // Sort by new
    for json in search_both(&anon, "sort=new").await {
        assert_eq!(json.meta.total, 4);
        assert_eq!(json.crates[0].name, "bar_sort");
        assert_eq!(json.crates[1].name, "other_sort");
        assert_eq!(json.crates[2].name, "baz_sort");
        assert_eq!(json.crates[3].name, "foo_sort");
    }
    let (resp, calls) = page_with_seek(&anon, "sort=new").await;
    assert_eq!(resp[0].crates[0].name, "bar_sort");
    assert_eq!(resp[1].crates[0].name, "other_sort");
    assert_eq!(resp[2].crates[0].name, "baz_sort");
    assert_eq!(resp[3].crates[0].name, "foo_sort");
    assert_eq!(resp[3].meta.total, 4);
    assert_eq!(calls, 5);

    // Sort by alpha with query
    // ordering (exact match desc, name asc)
    let query = "sort=alpha&q=bar_sort";
    let (resp, calls) = page_with_seek(&anon, query).await;
    for json in search_both(&anon, query).await {
        assert_eq!(json.meta.total, 3);
        assert_eq!(resp[0].crates[0].name, "bar_sort");
        assert_eq!(resp[1].crates[0].name, "baz_sort");
        assert_eq!(resp[2].crates[0].name, "foo_sort");
    }
    assert_eq!(calls, 4);

    let query = "sort=alpha&q=sort";
    let (resp, calls) = page_with_seek(&anon, query).await;
    for json in search_both(&anon, query).await {
        assert_eq!(json.meta.total, 4);
        assert_eq!(resp[0].crates[0].name, "bar_sort");
        assert_eq!(resp[1].crates[0].name, "baz_sort");
        assert_eq!(resp[2].crates[0].name, "foo_sort");
        assert_eq!(resp[3].crates[0].name, "other_sort");
    }
    assert_eq!(calls, 5);

    // Sort by relevance
    // ordering (exact match desc, rank desc, name asc)
    let query = "q=foo_sort";
    let (resp, calls) = page_with_seek(&anon, query).await;
    for json in search_both(&anon, query).await {
        assert_eq!(json.meta.total, 3);
        assert_eq!(resp[0].crates[0].name, "foo_sort");
        // same rank, by name asc
        assert_eq!(resp[1].crates[0].name, "bar_sort");
        assert_eq!(resp[2].crates[0].name, "baz_sort");
    }
    assert_eq!(calls, 4);
    let ranks = querystring_rank(&mut conn, "foo_sort").await;
    assert_eq!(ranks.get("bar_sort"), ranks.get("baz_sort"));

    // Add query containing a space to ensure tsquery works
    // "foo_sort" and "foo sort" would generate same tsquery
    let query = "q=foo%20sort";
    let (resp, calls) = page_with_seek(&anon, query).await;
    for json in search_both(&anon, query).await {
        assert_eq!(json.meta.total, 3);
        assert_eq!(resp[0].crates[0].name, "foo_sort");
        // same rank, by name asc
        assert_eq!(resp[1].crates[0].name, "bar_sort");
        assert_eq!(resp[2].crates[0].name, "baz_sort");
    }
    assert_eq!(calls, 4);
    let ranks = querystring_rank(&mut conn, "foo%20sort").await;
    assert_eq!(ranks.get("bar_sort"), ranks.get("baz_sort"));

    let query = "q=sort";
    let (resp, calls) = page_with_seek(&anon, query).await;
    for json in search_both(&anon, query).await {
        assert_eq!(json.meta.total, 4);
        // by rank desc (items with more "sort" should have a hider rank value)
        assert_eq!(resp[0].crates[0].name, "baz_sort");
        assert_eq!(resp[1].crates[0].name, "bar_sort");
        assert_eq!(resp[2].crates[0].name, "foo_sort");
        assert_eq!(resp[3].crates[0].name, "other_sort");
    }
    assert_eq!(calls, 5);
    let ranks = querystring_rank(&mut conn, "sort").await;
    assert_eq!(
        ranks.keys().collect::<Vec<_>>(),
        ["baz_sort", "bar_sort", "foo_sort", "other_sort"]
    );

    // Test for bug with showing null results first when sorting
    // by descending downloads
    for json in search_both(&anon, "sort=recent-downloads").await {
        assert_eq!(json.meta.total, 4);
        assert_eq!(json.crates[0].name, "baz_sort");
        assert_eq!(json.crates[1].name, "foo_sort");
        assert_eq!(json.crates[2].name, "bar_sort");
        assert_eq!(json.crates[3].name, "other_sort");
    }
    let (resp, calls) = page_with_seek(&anon, "sort=recent-downloads").await;
    assert_eq!(resp[0].crates[0].name, "baz_sort");
    assert_eq!(resp[1].crates[0].name, "foo_sort");
    assert_eq!(resp[2].crates[0].name, "bar_sort");
    assert_eq!(resp[3].crates[0].name, "other_sort");
    assert_eq!(resp[3].meta.total, 4);
    assert_eq!(calls, 5);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[allow(clippy::cognitive_complexity)]
async fn ignore_exact_match_on_queries_with_sort() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    let krate1 = CrateBuilder::new("foo_sort", user.id)
        .description("bar_sort baz_sort const")
        .downloads(50)
        .recent_downloads(50)
        .expect_build(&mut conn)
        .await;

    let krate2 = CrateBuilder::new("bar_sort", user.id)
        .description("foo_sort baz_sort foo_sort baz_sort const")
        .downloads(3333)
        .recent_downloads(0)
        .expect_build(&mut conn)
        .await;

    let krate3 = CrateBuilder::new("baz_sort", user.id)
        .description("foo_sort bar_sort foo_sort bar_sort foo_sort bar_sort const")
        .downloads(100_000)
        .recent_downloads(10)
        .expect_build(&mut conn)
        .await;

    let krate4 = CrateBuilder::new("other_sort", user.id)
        .description("other_sort const")
        .downloads(999_999)
        .expect_build(&mut conn)
        .await;

    // Set the created at column for each crate
    update(&krate1)
        .set(crates::created_at.eq(now.into_sql::<Timestamptz>() - 4.weeks()))
        .execute(&mut conn)
        .await?;
    update(&krate2)
        .set(crates::created_at.eq(now.into_sql::<Timestamptz>() - 1.weeks()))
        .execute(&mut conn)
        .await?;
    update(&krate3)
        .set(crates::created_at.eq(now.into_sql::<Timestamptz>() - 2.weeks()))
        .execute(&mut conn)
        .await?;
    update(&krate4)
        .set(crates::created_at.eq(now.into_sql::<Timestamptz>() - 3.weeks()))
        .execute(&mut conn)
        .await?;

    // Set the updated at column for each crate
    update(&krate1)
        .set(crates::updated_at.eq(now.into_sql::<Timestamptz>() - 3.weeks()))
        .execute(&mut conn)
        .await?;
    update(&krate2)
        .set(crates::updated_at.eq(now.into_sql::<Timestamptz>() - 5.days()))
        .execute(&mut conn)
        .await?;
    update(&krate3)
        .set(crates::updated_at.eq(now.into_sql::<Timestamptz>() - 10.seconds()))
        .execute(&mut conn)
        .await?;
    update(&krate4)
        .set(crates::updated_at.eq(now))
        .execute(&mut conn)
        .await?;

    // Sort by downloads, order always the same no matter the crate name query
    for json in search_both(&anon, "q=foo_sort&sort=downloads").await {
        assert_eq!(json.meta.total, 3);
        assert_eq!(json.crates[0].name, "baz_sort");
        assert_eq!(json.crates[1].name, "bar_sort");
        assert_eq!(json.crates[2].name, "foo_sort");
    }

    for json in search_both(&anon, "q=bar_sort&sort=downloads").await {
        assert_eq!(json.meta.total, 3);
        assert_eq!(json.crates[0].name, "baz_sort");
        assert_eq!(json.crates[1].name, "bar_sort");
        assert_eq!(json.crates[2].name, "foo_sort");
    }

    for json in search_both(&anon, "q=baz_sort&sort=downloads").await {
        assert_eq!(json.meta.total, 3);
        assert_eq!(json.crates[0].name, "baz_sort");
        assert_eq!(json.crates[1].name, "bar_sort");
        assert_eq!(json.crates[2].name, "foo_sort");
    }

    for json in search_both(&anon, "q=const&sort=downloads").await {
        assert_eq!(json.meta.total, 4);
        assert_eq!(json.crates[0].name, "other_sort");
        assert_eq!(json.crates[1].name, "baz_sort");
        assert_eq!(json.crates[2].name, "bar_sort");
        assert_eq!(json.crates[3].name, "foo_sort");
    }

    // Sort by recent-downloads, order always the same no matter the crate name query
    for json in search_both(&anon, "q=bar_sort&sort=recent-downloads").await {
        assert_eq!(json.meta.total, 3);
        assert_eq!(json.crates[0].name, "foo_sort");
        assert_eq!(json.crates[1].name, "baz_sort");
        assert_eq!(json.crates[2].name, "bar_sort");
    }

    // Test for bug with showing null results first when sorting
    // by descending downloads
    for json in search_both(&anon, "sort=recent-downloads").await {
        assert_eq!(json.meta.total, 4);
        assert_eq!(json.crates[0].name, "foo_sort");
        assert_eq!(json.crates[1].name, "baz_sort");
        assert_eq!(json.crates[2].name, "bar_sort");
        assert_eq!(json.crates[3].name, "other_sort");
    }

    // Sort by recent-updates
    for json in search_both(&anon, "q=bar_sort&sort=recent-updates").await {
        assert_eq!(json.meta.total, 3);
        assert_eq!(json.crates[0].name, "baz_sort");
        assert_eq!(json.crates[1].name, "bar_sort");
        assert_eq!(json.crates[2].name, "foo_sort");
    }

    // Sort by new
    for json in search_both(&anon, "q=bar_sort&sort=new").await {
        assert_eq!(json.meta.total, 3);
        assert_eq!(json.crates[0].name, "bar_sort");
        assert_eq!(json.crates[1].name, "baz_sort");
        assert_eq!(json.crates[2].name, "foo_sort");
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn multiple_ids() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    CrateBuilder::new("foo", user.id)
        .expect_build(&mut conn)
        .await;
    CrateBuilder::new("bar", user.id)
        .expect_build(&mut conn)
        .await;
    CrateBuilder::new("baz", user.id)
        .expect_build(&mut conn)
        .await;
    CrateBuilder::new("other", user.id)
        .expect_build(&mut conn)
        .await;

    let query = "ids[]=foo&ids[]=bar&ids[]=baz&ids[]=baz&ids[]=unknown";
    for json in search_both(&anon, query).await {
        assert_eq!(json.meta.total, 3);
        assert_eq!(json.crates[0].name, "bar");
        assert_eq!(json.crates[1].name, "baz");
        assert_eq!(json.crates[2].name, "foo");
    }

    let response = anon.search(&format!("{query}&per_page=1&page=2")).await;
    assert_snapshot!(response.meta.prev_page.unwrap(), @"?ids%5B%5D=foo&ids%5B%5D=bar&ids%5B%5D=baz&ids%5B%5D=baz&ids%5B%5D=unknown&per_page=1&page=1");
    assert_snapshot!(response.meta.next_page.unwrap(), @"?ids%5B%5D=foo&ids%5B%5D=bar&ids%5B%5D=baz&ids%5B%5D=baz&ids%5B%5D=unknown&per_page=1&page=3");

    let response = anon.search(&format!("{query}&per_page=1")).await;
    assert_snapshot!(response.meta.next_page.unwrap(), @"?ids%5B%5D=foo&ids%5B%5D=bar&ids%5B%5D=baz&ids%5B%5D=baz&ids%5B%5D=unknown&per_page=1&seek=Mg");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn loose_search_order() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    // exact match should be first
    let one = CrateBuilder::new("temp", user.id)
        .readme("readme")
        .description("description")
        .keyword("kw1")
        .expect_build(&mut conn)
        .await;
    // temp_udp should match second because of _
    let two = CrateBuilder::new("temp_utp", user.id)
        .readme("readme")
        .description("description")
        .keyword("kw1")
        .expect_build(&mut conn)
        .await;
    // evalrs should match 3rd because of readme
    let three = CrateBuilder::new("evalrs", user.id)
        .readme("evalrs_temp evalrs_temp evalrs_temp")
        .description("description")
        .keyword("kw1")
        .expect_build(&mut conn)
        .await;
    // tempfile should appear 4th
    let four = CrateBuilder::new("tempfile", user.id)
        .readme("readme")
        .description("description")
        .keyword("kw1")
        .expect_build(&mut conn)
        .await;

    let ordered = vec![one, two, three, four];

    for search_temp in search_both(&anon, "q=temp").await {
        assert_eq!(search_temp.meta.total, 4);
        assert_eq!(search_temp.crates.len(), 4);
        for (lhs, rhs) in search_temp.crates.iter().zip(&ordered) {
            assert_eq!(lhs.name, rhs.name);
        }
    }

    for search_temp in search_both(&anon, "q=te").await {
        assert_eq!(search_temp.meta.total, 3);
        assert_eq!(search_temp.crates.len(), 3);
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn index_include_yanked() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    CrateBuilder::new("unyanked", user.id)
        .version(VersionBuilder::new("1.0.0"))
        .version(VersionBuilder::new("2.0.0"))
        .expect_build(&mut conn)
        .await;

    CrateBuilder::new("newest_yanked", user.id)
        .version(VersionBuilder::new("1.0.0"))
        .version(VersionBuilder::new("2.0.0").yanked(true))
        .expect_build(&mut conn)
        .await;

    CrateBuilder::new("oldest_yanked", user.id)
        .version(VersionBuilder::new("1.0.0").yanked(true))
        .version(VersionBuilder::new("2.0.0"))
        .expect_build(&mut conn)
        .await;

    CrateBuilder::new("all_yanked", user.id)
        .version(VersionBuilder::new("1.0.0").yanked(true))
        .version(VersionBuilder::new("2.0.0").yanked(true))
        .expect_build(&mut conn)
        .await;

    // Include fully yanked (all versions were yanked) crates
    for json in search_both(&anon, "include_yanked=yes&sort=alphabetical").await {
        assert_eq!(json.meta.total, 4);
        assert_eq!(json.crates[0].name, "all_yanked");
        assert_eq!(json.crates[1].name, "newest_yanked");
        assert_eq!(json.crates[2].name, "oldest_yanked");
        assert_eq!(json.crates[3].name, "unyanked");

        assert_eq!(
            default_versions_iter(&json.crates)
                .flat_map(|s| s.as_deref())
                .zip(yanked_iter(&json.crates).cloned())
                .collect::<Vec<_>>(),
            [
                ("2.0.0", true),
                ("1.0.0", false),
                ("2.0.0", false),
                ("2.0.0", false),
            ]
        );
    }

    // Do not include fully yanked (all versions were yanked) crates
    for json in search_both(&anon, "include_yanked=no&sort=alphabetical").await {
        assert_eq!(json.meta.total, 3);
        assert_eq!(json.crates[0].name, "newest_yanked");
        assert_eq!(json.crates[1].name, "oldest_yanked");
        assert_eq!(json.crates[2].name, "unyanked");
        assert_eq!(
            default_versions_iter(&json.crates)
                .flat_map(|s| s.as_deref())
                .zip(yanked_iter(&json.crates).cloned())
                .collect::<Vec<_>>(),
            [("1.0.0", false), ("2.0.0", false), ("2.0.0", false),]
        );
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn yanked_versions_are_not_considered_for_max_version() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    CrateBuilder::new("foo_yanked_version", user.id)
        .description("foo")
        .version("1.0.0")
        .version(VersionBuilder::new("1.1.0").yanked(true))
        .expect_build(&mut conn)
        .await;

    for json in search_both(&anon, "q=foo").await {
        assert_eq!(json.meta.total, 1);
        assert_eq!(json.crates[0].default_version, Some("1.0.0".into()));
        assert!(!json.crates[0].yanked);
        assert_eq!(json.crates[0].max_version, "1.0.0");
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn max_stable_version() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    CrateBuilder::new("foo", user.id)
        .description("foo")
        .version("0.3.0")
        .version("1.0.0")
        .version(VersionBuilder::new("1.1.0").yanked(true))
        .version("2.0.0-beta.1")
        .version("0.3.1")
        .expect_build(&mut conn)
        .await;

    for json in search_both(&anon, "q=foo").await {
        assert_eq!(json.meta.total, 1);
        assert_eq!(json.crates[0].default_version, Some("1.0.0".into()));
        assert!(!json.crates[0].yanked);
        assert_eq!(json.crates[0].max_stable_version, Some("1.0.0".to_string()));
    }

    Ok(())
}

/// Given two crates, one with downloads less than 90 days ago, the
/// other with all downloads greater than 90 days ago, check that
/// the order returned is by recent downloads, descending. Check
/// also that recent download counts are returned in recent_downloads,
/// and total downloads counts are returned in downloads, and that
/// these numbers do not overlap.
#[tokio::test(flavor = "multi_thread")]
async fn test_recent_download_count() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    // More than 90 days ago
    CrateBuilder::new("green_ball", user.id)
        .description("For fetching")
        .downloads(10)
        .recent_downloads(0)
        .expect_build(&mut conn)
        .await;

    CrateBuilder::new("sweet_potato_snack", user.id)
        .description("For when better than usual")
        .downloads(5)
        .recent_downloads(2)
        .expect_build(&mut conn)
        .await;

    for json in search_both(&anon, "sort=recent-downloads").await {
        assert_eq!(json.meta.total, 2);

        assert_eq!(json.crates[0].name, "sweet_potato_snack");
        assert_eq!(json.crates[1].name, "green_ball");

        assert_eq!(json.crates[0].recent_downloads, Some(2));
        assert_eq!(json.crates[0].downloads, 5);

        assert_eq!(json.crates[1].recent_downloads, Some(0));
        assert_eq!(json.crates[1].downloads, 10);
    }

    Ok(())
}

/// Given one crate with zero downloads, check that the crate
/// still shows up in index results, but that it displays 0
/// for both recent downloads and downloads.
#[tokio::test(flavor = "multi_thread")]
async fn test_zero_downloads() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    // More than 90 days ago
    CrateBuilder::new("green_ball", user.id)
        .description("For fetching")
        .downloads(0)
        .recent_downloads(0)
        .expect_build(&mut conn)
        .await;

    for json in search_both(&anon, "sort=recent-downloads").await {
        assert_eq!(json.meta.total, 1);
        assert_eq!(json.crates[0].name, "green_ball");
        assert_eq!(json.crates[0].recent_downloads, Some(0));
        assert_eq!(json.crates[0].downloads, 0);
    }

    Ok(())
}

/// Given two crates, one with more all-time downloads, the other with
/// more downloads in the past 90 days, check that the index page for
/// categories and keywords is sorted by recent downloads by default.
#[tokio::test(flavor = "multi_thread")]
async fn test_default_sort_recent() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    // More than 90 days ago
    let green_crate = CrateBuilder::new("green_ball", user.id)
        .description("For fetching")
        .keyword("dog")
        .downloads(10)
        .recent_downloads(10)
        .expect_build(&mut conn)
        .await;

    let potato_crate = CrateBuilder::new("sweet_potato_snack", user.id)
        .description("For when better than usual")
        .keyword("dog")
        .downloads(20)
        .recent_downloads(0)
        .expect_build(&mut conn)
        .await;

    // test that index for keywords is sorted by recent_downloads
    // by default
    for json in search_both(&anon, "keyword=dog").await {
        assert_eq!(json.meta.total, 2);

        assert_eq!(json.crates[0].name, "green_ball");
        assert_eq!(json.crates[1].name, "sweet_potato_snack");

        assert_eq!(json.crates[0].recent_downloads, Some(10));
        assert_eq!(json.crates[0].downloads, 10);

        assert_eq!(json.crates[1].recent_downloads, Some(0));
        assert_eq!(json.crates[1].downloads, 20);
    }

    insert_into(categories::table)
        .values(new_category("Animal", "animal", "animal crates"))
        .execute(&mut conn)
        .await?;

    Category::update_crate(&mut conn, green_crate.id, &["animal"]).await?;
    Category::update_crate(&mut conn, potato_crate.id, &["animal"]).await?;

    // test that index for categories is sorted by recent_downloads
    // by default
    for json in search_both(&anon, "category=animal").await {
        assert_eq!(json.meta.total, 2);

        assert_eq!(json.crates[0].name, "green_ball");
        assert_eq!(json.crates[1].name, "sweet_potato_snack");

        assert_eq!(json.crates[0].recent_downloads, Some(10));
        assert_eq!(json.crates[0].downloads, 10);

        assert_eq!(json.crates[1].recent_downloads, Some(0));
        assert_eq!(json.crates[1].downloads, 20);
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn pagination_links_included_if_applicable() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    CrateBuilder::new("pagination_links_1", user.id)
        .expect_build(&mut conn)
        .await;
    CrateBuilder::new("pagination_links_2", user.id)
        .expect_build(&mut conn)
        .await;
    CrateBuilder::new("pagination_links_3", user.id)
        .expect_build(&mut conn)
        .await;

    // This uses a filter (`page=n`) to disable seek-based pagination, as seek-based pagination
    // does not return page numbers.

    let page1 = anon.search("letter=p&page=1&per_page=1").await;
    let page2 = anon.search("letter=p&page=2&per_page=1").await;
    let page3 = anon.search("letter=p&page=3&per_page=1").await;
    let page4 = anon.search("letter=p&page=4&per_page=1").await;

    assert_eq!(
        Some("?letter=p&per_page=1&page=2".to_string()),
        page1.meta.next_page
    );
    assert_eq!(None, page1.meta.prev_page);
    assert_eq!(
        Some("?letter=p&per_page=1&page=3".to_string()),
        page2.meta.next_page
    );
    assert_eq!(
        Some("?letter=p&per_page=1&page=1".to_string()),
        page2.meta.prev_page
    );
    assert_eq!(None, page4.meta.next_page);
    assert_eq!(
        Some("?letter=p&per_page=1&page=2".to_string()),
        page3.meta.prev_page
    );
    assert!(
        [page1.meta.total, page2.meta.total, page3.meta.total]
            .iter()
            .all(|w| *w == 3)
    );
    assert_eq!(page4.meta.total, 0);
    for p in [page1, page2, page3, page4] {
        assert!(default_versions_iter(&p.crates).all(Option::is_some));
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn seek_based_pagination() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    CrateBuilder::new("pagination_links_1", user.id)
        .expect_build(&mut conn)
        .await;
    CrateBuilder::new("pagination_links_2", user.id)
        .expect_build(&mut conn)
        .await;
    CrateBuilder::new("pagination_links_3", user.id)
        .expect_build(&mut conn)
        .await;

    let mut url = Some("?per_page=1".to_string());
    let mut results = Vec::new();
    let mut calls = 0;
    while let Some(current_url) = url.take() {
        let resp = anon.search(current_url.trim_start_matches('?')).await;
        calls += 1;

        results.append(
            &mut resp
                .crates
                .iter()
                .map(|res| res.name.clone())
                .collect::<Vec<_>>(),
        );

        if let Some(new_url) = resp.meta.next_page {
            assert_that!(resp.crates, len(eq(1)));
            url = Some(new_url);
            assert_eq!(resp.meta.total, 3);
            assert!(default_versions_iter(&resp.crates).all(Option::is_some));
        } else {
            assert_that!(resp.crates, empty());
            assert_eq!(resp.meta.total, 0);
        }

        assert_eq!(resp.meta.prev_page, None);
    }

    assert_eq!(calls, 4);
    assert_eq!(
        vec![
            "pagination_links_1",
            "pagination_links_2",
            "pagination_links_3"
        ],
        results
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_pages_work_even_with_seek_based_pagination() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    CrateBuilder::new("pagination_links_1", user.id)
        .expect_build(&mut conn)
        .await;
    CrateBuilder::new("pagination_links_2", user.id)
        .expect_build(&mut conn)
        .await;
    CrateBuilder::new("pagination_links_3", user.id)
        .expect_build(&mut conn)
        .await;

    // The next_page returned by the request is seek-based
    let first = anon.search("per_page=1").await;
    assert!(first.meta.next_page.unwrap().contains("seek="));
    assert_eq!(first.meta.total, 3);
    assert!(default_versions_iter(&first.crates).all(Option::is_some));

    // Calling with page=2 will revert to offset-based pagination
    let second = anon.search("page=2&per_page=1").await;
    assert!(second.meta.next_page.unwrap().contains("page=3"));
    assert_eq!(second.meta.total, 3);
    assert!(default_versions_iter(&second.crates).all(Option::is_some));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_params_with_null_bytes() {
    let (_app, anon, _cookie) = TestApp::init().with_user().await;

    for name in ["q", "category", "all_keywords", "keyword", "letter"] {
        let response = anon.get::<()>(&format!("/api/v1/crates?{name}=%00")).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_json_snapshot!(response.json());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_seek_parameter() {
    let (_app, anon, _cookie) = TestApp::init().with_user().await;

    let response = anon.get::<()>("/api/v1/crates?seek=broken").await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid seek parameter"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn pagination_parameters_only_accept_integers() {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    CrateBuilder::new("pagination_links_1", user.id)
        .expect_build(&mut conn)
        .await;
    CrateBuilder::new("pagination_links_2", user.id)
        .expect_build(&mut conn)
        .await;
    CrateBuilder::new("pagination_links_3", user.id)
        .expect_build(&mut conn)
        .await;

    let response = anon
        .get_with_query::<()>("/api/v1/crates", "page=1&per_page=100%22%EF%BC%8Cexception")
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to deserialize query string: per_page: invalid digit found in string"}]}"#);

    let response = anon
        .get_with_query::<()>("/api/v1/crates", "page=100%22%EF%BC%8Cexception&per_page=1")
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to deserialize query string: page: invalid digit found in string"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn crates_by_user_id() {
    let (app, _, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let id = user.as_model().id;

    CrateBuilder::new("foo_my_packages", id)
        .expect_build(&mut conn)
        .await;

    for response in search_both_by_user_id(&user, id).await {
        assert_eq!(response.crates.len(), 1);
        assert_eq!(response.meta.total, 1);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn crates_by_user_id_not_including_deleted_owners() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    let krate = CrateBuilder::new("foo_my_packages", user.id)
        .expect_build(&mut conn)
        .await;
    krate.owner_remove(&mut conn, "foo").await.unwrap();

    for response in search_both_by_user_id(&anon, user.id).await {
        assert_eq!(response.crates.len(), 0);
        assert_eq!(response.meta.total, 0);
    }

    Ok(())
}

static PAGE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"((?:^page|&page|\?page)=\d+)").unwrap());

// search with both offset-based (prepend with `page=1` query) and seek-based pagination
async fn search_both<U: RequestHelper>(anon: &U, query: &str) -> [crate::tests::CrateList; 2] {
    if PAGE_RE.is_match(query) {
        panic!("url already contains page param");
    }
    let (offset, seek) = (
        anon.search(&format!("page=1&{query}")).await,
        anon.search(query).await,
    );
    assert!(
        offset
            .meta
            .next_page
            .as_deref()
            .unwrap_or("page=2")
            .contains("page=2")
    );
    assert!(
        seek.meta
            .next_page
            .as_deref()
            .unwrap_or("seek=")
            .contains("seek=")
    );
    assert!(default_versions_iter(&offset.crates).all(Option::is_some));
    assert!(default_versions_iter(&seek.crates).all(Option::is_some));
    [offset, seek]
}

async fn search_both_by_user_id<U: RequestHelper>(
    anon: &U,
    id: i32,
) -> [crate::tests::CrateList; 2] {
    let url = format!("user_id={id}");
    search_both(anon, &url).await
}

async fn page_with_seek<U: RequestHelper>(
    anon: &U,
    query: &str,
) -> (Vec<crate::tests::CrateList>, i32) {
    let mut url = Some(format!("?per_page=1&{query}"));
    let mut results = Vec::new();
    let mut calls = 0;
    while let Some(current_url) = url.take() {
        let resp = anon.search(current_url.trim_start_matches('?')).await;
        calls += 1;
        if calls > 200 {
            panic!("potential infinite loop detected!")
        }

        if let Some(ref new_url) = resp.meta.next_page {
            assert!(new_url.contains("seek="));
            assert_that!(resp.crates, len(eq(1)));
            url = Some(new_url.to_owned());
            assert_ne!(resp.meta.total, 0);
            assert!(default_versions_iter(&resp.crates).all(Option::is_some));
        } else {
            assert_that!(resp.crates, empty());
            assert_eq!(resp.meta.total, 0);
        }
        results.push(resp);
    }
    (results, calls)
}

fn default_versions_iter(
    crates: &[crate::tests::EncodableCrate],
) -> impl Iterator<Item = &Option<String>> {
    crates.iter().map(|c| &c.default_version)
}

fn yanked_iter(crates: &[crate::tests::EncodableCrate]) -> impl Iterator<Item = &bool> {
    crates.iter().map(|c| &c.yanked)
}

async fn querystring_rank(
    conn: &mut diesel_async::AsyncPgConnection,
    q: &str,
) -> indexmap::IndexMap<String, f32> {
    use diesel_full_text_search::configuration::TsConfigurationByName;
    use diesel_full_text_search::{plainto_tsquery_with_search_config, ts_rank_cd};
    use futures_util::TryStreamExt;
    use futures_util::future::ready;

    let tsquery = plainto_tsquery_with_search_config(TsConfigurationByName("english"), q);
    let rank = ts_rank_cd(crates::textsearchable_index_col, tsquery);
    crates::table
        .select((crates::name, rank))
        .order_by(rank.desc())
        .load_stream::<(String, f32)>(conn)
        .await
        .unwrap()
        .try_fold(indexmap::IndexMap::new(), |mut map, (name, id)| {
            map.insert(name, id);
            ready(Ok(map))
        })
        .await
        .unwrap()
}
