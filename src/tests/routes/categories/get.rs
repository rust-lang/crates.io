use crate::models::Category;
use crate::tests::builders::CrateBuilder;
use crate::tests::new_category;
use crate::tests::util::{MockAnonymousUser, RequestHelper, TestApp};
use crates_io_database::schema::categories;
use diesel::insert_into;
use diesel_async::RunQueryDsl;
use insta::assert_json_snapshot;
use serde_json::Value;

#[tokio::test(flavor = "multi_thread")]
async fn show() -> anyhow::Result<()> {
    let (app, anon) = TestApp::init().empty().await;
    let mut conn = app.db_conn().await;

    let url = "/api/v1/categories/foo-bar";

    // Return not found if a category doesn't exist
    anon.get(url).await.assert_not_found();

    // Create a category and a subcategory
    let cats = vec![
        new_category("Foo Bar", "foo-bar", "Foo Bar crates"),
        new_category("Foo Bar::Baz", "foo-bar::baz", "Baz crates"),
    ];

    insert_into(categories::table)
        .values(cats)
        .execute(&mut conn)
        .await?;

    // The category and its subcategories should be in the json
    let json: Value = anon.get(url).await.good();
    assert_json_snapshot!(json, {
        ".**.created_at" => "[datetime]",
    });

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[allow(clippy::cognitive_complexity)]
async fn update_crate() -> anyhow::Result<()> {
    // Convenience function to get the number of crates in a category
    async fn count(anon: &MockAnonymousUser, category: &str) -> usize {
        let json = anon.show_category(category).await;
        json.category.crates_cnt as usize
    }

    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    let cats = vec![
        new_category("cat1", "cat1", "Category 1 crates"),
        new_category("Category 2", "category-2", "Category 2 crates"),
    ];

    assert_ok!(
        insert_into(categories::table)
            .values(cats)
            .execute(&mut conn)
            .await
    );

    let krate = CrateBuilder::new("foo_crate", user.id)
        .expect_build(&mut conn)
        .await;

    // Updating with no categories has no effect
    Category::update_crate(&mut conn, krate.id, &[]).await?;
    assert_eq!(count(&anon, "cat1").await, 0);
    assert_eq!(count(&anon, "category-2").await, 0);

    // Happy path adding one category
    Category::update_crate(&mut conn, krate.id, &["cat1"]).await?;
    assert_eq!(count(&anon, "cat1").await, 1);
    assert_eq!(count(&anon, "category-2").await, 0);

    // Replacing one category with another
    Category::update_crate(&mut conn, krate.id, &["category-2"]).await?;
    assert_eq!(count(&anon, "cat1").await, 0);
    assert_eq!(count(&anon, "category-2").await, 1);

    // Removing one category
    Category::update_crate(&mut conn, krate.id, &[]).await?;
    assert_eq!(count(&anon, "cat1").await, 0);
    assert_eq!(count(&anon, "category-2").await, 0);

    // Adding 2 categories
    Category::update_crate(&mut conn, krate.id, &["cat1", "category-2"]).await?;
    assert_eq!(count(&anon, "cat1").await, 1);
    assert_eq!(count(&anon, "category-2").await, 1);

    // Removing all categories
    Category::update_crate(&mut conn, krate.id, &[]).await?;
    assert_eq!(count(&anon, "cat1").await, 0);
    assert_eq!(count(&anon, "category-2").await, 0);

    // Attempting to add one valid category and one invalid category
    let invalid_categories =
        Category::update_crate(&mut conn, krate.id, &["cat1", "catnope"]).await?;
    assert_eq!(invalid_categories, vec!["catnope"]);
    assert_eq!(count(&anon, "cat1").await, 1);
    assert_eq!(count(&anon, "category-2").await, 0);

    // Does not add the invalid category to the category list
    // (unlike the behavior of keywords)
    let json = anon.show_category_list().await;
    assert_eq!(json.categories.len(), 2);
    assert_eq!(json.meta.total, 2);

    // Attempting to add a category by display text; must use slug
    Category::update_crate(&mut conn, krate.id, &["Category 2"]).await?;
    assert_eq!(count(&anon, "cat1").await, 0);
    assert_eq!(count(&anon, "category-2").await, 0);

    // Add a category and its subcategory
    assert_ok!(
        insert_into(categories::table)
            .values(new_category("cat1::bar", "cat1::bar", "bar crates"))
            .execute(&mut conn)
            .await
    );

    Category::update_crate(&mut conn, krate.id, &["cat1", "cat1::bar"]).await?;

    assert_eq!(count(&anon, "cat1").await, 1);
    assert_eq!(count(&anon, "cat1::bar").await, 1);
    assert_eq!(count(&anon, "category-2").await, 0);

    Ok(())
}
