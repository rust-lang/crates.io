use crate::models::Crate;
use crate::schema::*;
use chrono::{DateTime, Utc};
use diesel::dsl;
use diesel::prelude::*;
use diesel::sql_types::Text;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use futures_util::FutureExt;
use futures_util::future::BoxFuture;

#[derive(Clone, Identifiable, HasQuery, QueryableByName, Debug)]
#[diesel(table_name = categories)]
pub struct Category {
    pub id: i32,
    pub category: String,
    pub slug: String,
    pub description: String,
    pub crates_cnt: i32,
    pub created_at: DateTime<Utc>,
}

type WithSlug<'a> = dsl::Eq<categories::slug, crates_io_diesel_helpers::lower<Text, &'a str>>;

#[derive(Associations, Insertable, Identifiable, Debug, Clone, Copy)]
#[diesel(
    table_name = crates_categories,
    check_for_backend(diesel::pg::Pg),
    primary_key(crate_id, category_id),
    belongs_to(Category),
    belongs_to(Crate),
)]
pub struct CrateCategory {
    crate_id: i32,
    category_id: i32,
}

impl Category {
    pub fn with_slug(slug: &str) -> WithSlug<'_> {
        categories::slug.eq(crates_io_diesel_helpers::lower(slug))
    }

    #[dsl::auto_type(no_type_alias)]
    pub fn by_slug<'a>(slug: &'a str) -> _ {
        let filter: WithSlug<'a> = Self::with_slug(slug);
        categories::table.filter(filter)
    }

    pub async fn update_crate(
        conn: &mut AsyncPgConnection,
        crate_id: i32,
        slugs: &[&str],
    ) -> QueryResult<Vec<String>> {
        conn.transaction(|conn| {
            async move {
                let categories: Vec<Category> = Category::query()
                    .filter(categories::slug.eq_any(slugs))
                    .load(conn)
                    .await?;

                let invalid_categories = slugs
                    .iter()
                    .filter(|s| !categories.iter().any(|c| c.slug == **s))
                    .map(ToString::to_string)
                    .collect();

                let crate_categories = categories
                    .iter()
                    .map(|c| CrateCategory {
                        category_id: c.id,
                        crate_id,
                    })
                    .collect::<Vec<_>>();

                diesel::delete(crates_categories::table)
                    .filter(crates_categories::crate_id.eq(crate_id))
                    .execute(conn)
                    .await?;

                diesel::insert_into(crates_categories::table)
                    .values(&crate_categories)
                    .execute(conn)
                    .await?;

                Ok(invalid_categories)
            }
            .scope_boxed()
        })
        .await
    }

    pub async fn count_toplevel(conn: &mut AsyncPgConnection) -> QueryResult<i64> {
        categories::table
            .filter(categories::category.not_like("%::%"))
            .count()
            .get_result(conn)
            .await
    }

    pub fn toplevel<'a>(
        conn: &mut AsyncPgConnection,
        sort: &'a str,
        limit: i64,
        offset: i64,
    ) -> BoxFuture<'a, QueryResult<Vec<Category>>> {
        use diesel::sql_types::Int8;

        let sort_sql = match sort {
            "crates" => "ORDER BY crates_cnt DESC",
            _ => "ORDER BY category ASC",
        };

        // Collect all the top-level categories and sum up the crates_cnt of
        // the crates in all subcategories
        diesel::sql_query(format!(include_str!("toplevel.sql"), sort_sql))
            .bind::<Int8, _>(limit)
            .bind::<Int8, _>(offset)
            .load(conn)
            .boxed()
    }

    pub fn subcategories(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> BoxFuture<'_, QueryResult<Vec<Category>>> {
        use diesel::sql_types::Text;

        diesel::sql_query(include_str!("subcategories.sql"))
            .bind::<Text, _>(&self.category)
            .load(conn)
            .boxed()
    }

    /// Gathers the parent categories from the top-level Category to the direct parent of this Category.
    /// Returns categories as a Vector in order of traversal, not including this Category.
    /// The intention is to be able to have slugs or parent categories arrayed in order, to
    /// offer the frontend, for examples, slugs to create links to each parent category in turn.
    pub async fn parent_categories(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> QueryResult<Vec<Category>> {
        use diesel::sql_types::Text;

        diesel::sql_query(include_str!("parent_categories.sql"))
            .bind::<Text, _>(&self.slug)
            .load(conn)
            .await
    }
}

/// Struct for inserting categories; only used in tests. Actual categories are inserted
/// in src/boot/categories.rs.
#[derive(Insertable, AsChangeset, Default, Debug)]
#[diesel(table_name = categories, check_for_backend(diesel::pg::Pg))]
pub struct NewCategory<'a> {
    pub category: &'a str,
    pub slug: &'a str,
    pub description: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crates_io_test_db::TestDatabase;
    use diesel_async::RunQueryDsl;

    #[tokio::test]
    async fn category_toplevel_excludes_subcategories() {
        use self::categories;

        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        diesel::insert_into(categories::table)
            .values(&vec![
                (
                    categories::category.eq("Cat 2"),
                    categories::slug.eq("cat2"),
                ),
                (
                    categories::category.eq("Cat 1"),
                    categories::slug.eq("cat1"),
                ),
                (
                    categories::category.eq("Cat 1::sub"),
                    categories::slug.eq("cat1::sub"),
                ),
            ])
            .execute(&mut conn)
            .await
            .unwrap();

        let cats = Category::toplevel(&mut conn, "", 10, 0)
            .await
            .unwrap()
            .into_iter()
            .map(|c| c.category)
            .collect::<Vec<_>>();
        let expected = vec!["Cat 1".to_string(), "Cat 2".to_string()];
        assert_eq!(expected, cats);
    }

    #[tokio::test]
    async fn category_toplevel_orders_by_crates_cnt_when_sort_given() {
        use self::categories;
        use crate::schema::crates;
        use crate::schema::crates_categories;

        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        diesel::insert_into(categories::table)
            .values(&vec![
                (
                    categories::category.eq("Cat 1"),
                    categories::slug.eq("cat1"),
                ),
                (
                    categories::category.eq("Cat 2"),
                    categories::slug.eq("cat2"),
                ),
                (
                    categories::category.eq("Cat 3"),
                    categories::slug.eq("cat3"),
                ),
            ])
            .execute(&mut conn)
            .await
            .unwrap();

        let _cat1: Category = Category::by_slug("cat1")
            .select(Category::as_select())
            .first(&mut conn)
            .await
            .unwrap();
        let cat2: Category = Category::by_slug("cat2")
            .select(Category::as_select())
            .first(&mut conn)
            .await
            .unwrap();
        let cat3: Category = Category::by_slug("cat3")
            .select(Category::as_select())
            .first(&mut conn)
            .await
            .unwrap();

        let insert_crate = |name: &'static str| {
            diesel::insert_into(crates::table)
                .values(crates::name.eq(name))
                .returning(crates::id)
        };

        // Cat 1: 0 crates
        // Cat 2: 2 crates
        let k1: i32 = insert_crate("k1").get_result(&mut conn).await.unwrap();
        let k2: i32 = insert_crate("k2").get_result(&mut conn).await.unwrap();
        // Cat 3: 1 crate
        let k3: i32 = insert_crate("k3").get_result(&mut conn).await.unwrap();

        diesel::insert_into(crates_categories::table)
            .values(&vec![
                (
                    crates_categories::crate_id.eq(k1),
                    crates_categories::category_id.eq(cat2.id),
                ),
                (
                    crates_categories::crate_id.eq(k2),
                    crates_categories::category_id.eq(cat2.id),
                ),
                (
                    crates_categories::crate_id.eq(k3),
                    crates_categories::category_id.eq(cat3.id),
                ),
            ])
            .execute(&mut conn)
            .await
            .unwrap();

        let cats = Category::toplevel(&mut conn, "crates", 10, 0)
            .await
            .unwrap()
            .into_iter()
            .map(|c| c.category)
            .collect::<Vec<_>>();
        let expected = vec![
            "Cat 2".to_string(),
            "Cat 3".to_string(),
            "Cat 1".to_string(),
        ];
        assert_eq!(expected, cats);
    }

    #[tokio::test]
    async fn category_toplevel_applies_limit_and_offset() {
        use self::categories;

        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        diesel::insert_into(categories::table)
            .values(&vec![
                (
                    categories::category.eq("Cat 1"),
                    categories::slug.eq("cat1"),
                ),
                (
                    categories::category.eq("Cat 2"),
                    categories::slug.eq("cat2"),
                ),
            ])
            .execute(&mut conn)
            .await
            .unwrap();

        let cats = Category::toplevel(&mut conn, "", 1, 0)
            .await
            .unwrap()
            .into_iter()
            .map(|c| c.category)
            .collect::<Vec<_>>();
        let expected = vec!["Cat 1".to_string()];
        assert_eq!(expected, cats);

        let cats = Category::toplevel(&mut conn, "", 1, 1)
            .await
            .unwrap()
            .into_iter()
            .map(|c| c.category)
            .collect::<Vec<_>>();
        let expected = vec!["Cat 2".to_string()];
        assert_eq!(expected, cats);
    }

    #[tokio::test]
    async fn category_toplevel_includes_subcategories_in_crate_cnt() {
        use self::categories;
        use crate::schema::crates;
        use crate::schema::crates_categories;

        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        diesel::insert_into(categories::table)
            .values(&vec![
                (
                    categories::category.eq("Cat 1"),
                    categories::slug.eq("cat1"),
                ),
                (
                    categories::category.eq("Cat 1::sub"),
                    categories::slug.eq("cat1::sub"),
                ),
                (
                    categories::category.eq("Cat 2"),
                    categories::slug.eq("cat2"),
                ),
                (
                    categories::category.eq("Cat 2::Sub 1"),
                    categories::slug.eq("cat2::sub1"),
                ),
                (
                    categories::category.eq("Cat 2::Sub 2"),
                    categories::slug.eq("cat2::sub2"),
                ),
                (
                    categories::category.eq("Cat 3"),
                    categories::slug.eq("cat3"),
                ),
            ])
            .execute(&mut conn)
            .await
            .unwrap();

        let mut crate_idx = 0;
        // Total crates insertion loop
        let slugs = vec![
            ("cat1", 1),
            ("cat1::sub", 2),
            ("cat2", 3),
            ("cat2::sub1", 4),
            ("cat2::sub2", 5),
            ("cat3", 6),
        ];

        for (slug, count) in slugs {
            let cat_id: i32 = Category::by_slug(slug)
                .select(categories::id)
                .first::<i32>(&mut conn)
                .await
                .unwrap();
            for _i in 0..count {
                crate_idx += 1;
                let name = format!("k_{}", crate_idx);
                let k_id: i32 = diesel::insert_into(crates::table)
                    .values(crates::name.eq(name))
                    .returning(crates::id)
                    .get_result(&mut conn)
                    .await
                    .unwrap();
                diesel::insert_into(crates_categories::table)
                    .values((
                        crates_categories::crate_id.eq(k_id),
                        crates_categories::category_id.eq(cat_id),
                    ))
                    .execute(&mut conn)
                    .await
                    .unwrap();
            }
        }

        let cats = Category::toplevel(&mut conn, "crates", 10, 0)
            .await
            .unwrap()
            .into_iter()
            .map(|c| (c.category, c.crates_cnt))
            .collect::<Vec<_>>();
        let expected = vec![
            ("Cat 2".to_string(), 12),
            ("Cat 3".to_string(), 6),
            ("Cat 1".to_string(), 3),
        ];
        assert_eq!(expected, cats);
    }

    #[tokio::test]
    async fn category_toplevel_applies_limit_and_offset_after_grouping() {
        use self::categories;
        use crate::schema::crates;
        use crate::schema::crates_categories;

        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        diesel::insert_into(categories::table)
            .values(&vec![
                (
                    categories::category.eq("Cat 1"),
                    categories::slug.eq("cat1"),
                ),
                (
                    categories::category.eq("Cat 1::sub"),
                    categories::slug.eq("cat1::sub"),
                ),
                (
                    categories::category.eq("Cat 2"),
                    categories::slug.eq("cat2"),
                ),
                (
                    categories::category.eq("Cat 2::Sub 1"),
                    categories::slug.eq("cat2::sub1"),
                ),
                (
                    categories::category.eq("Cat 2::Sub 2"),
                    categories::slug.eq("cat2::sub2"),
                ),
                (
                    categories::category.eq("Cat 3"),
                    categories::slug.eq("cat3"),
                ),
            ])
            .execute(&mut conn)
            .await
            .unwrap();

        let mut crate_idx = 0;
        let slugs = vec![
            ("cat1", 1),
            ("cat1::sub", 2),
            ("cat2", 3),
            ("cat2::sub1", 4),
            ("cat2::sub2", 5),
            ("cat3", 6),
        ];

        for (slug, count) in slugs {
            let cat_id: i32 = Category::by_slug(slug)
                .select(categories::id)
                .first::<i32>(&mut conn)
                .await
                .unwrap();
            for _ in 0..count {
                crate_idx += 1;
                let name = format!("k_{}", crate_idx);
                let k_id: i32 = diesel::insert_into(crates::table)
                    .values(crates::name.eq(name))
                    .returning(crates::id)
                    .get_result(&mut conn)
                    .await
                    .unwrap();
                diesel::insert_into(crates_categories::table)
                    .values((
                        crates_categories::crate_id.eq(k_id),
                        crates_categories::category_id.eq(cat_id),
                    ))
                    .execute(&mut conn)
                    .await
                    .unwrap();
            }
        }

        let cats = Category::toplevel(&mut conn, "crates", 2, 0)
            .await
            .unwrap()
            .into_iter()
            .map(|c| (c.category, c.crates_cnt))
            .collect::<Vec<_>>();
        let expected = vec![("Cat 2".to_string(), 12), ("Cat 3".to_string(), 6)];
        assert_eq!(expected, cats);

        let cats = Category::toplevel(&mut conn, "crates", 2, 1)
            .await
            .unwrap()
            .into_iter()
            .map(|c| (c.category, c.crates_cnt))
            .collect::<Vec<_>>();
        let expected = vec![("Cat 3".to_string(), 6), ("Cat 1".to_string(), 3)];
        assert_eq!(expected, cats);
    }

    #[tokio::test]
    async fn category_parent_categories_includes_path_to_node_with_count() {
        use self::categories;
        use crate::schema::crates;
        use crate::schema::crates_categories;

        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        diesel::insert_into(categories::table)
            .values(&vec![
                (
                    categories::category.eq("Cat 1"),
                    categories::slug.eq("cat1"),
                ),
                (
                    categories::category.eq("Cat 1::sub1"),
                    categories::slug.eq("cat1::sub1"),
                ),
                (
                    categories::category.eq("Cat 1::sub2"),
                    categories::slug.eq("cat1::sub2"),
                ),
                (
                    categories::category.eq("Cat 1::sub1::subsub1"),
                    categories::slug.eq("cat1::sub1::subsub1"),
                ),
                (
                    categories::category.eq("Cat 2"),
                    categories::slug.eq("cat2"),
                ),
                (
                    categories::category.eq("Cat 2::Sub 1"),
                    categories::slug.eq("cat2::sub1"),
                ),
                (
                    categories::category.eq("Cat 2::Sub 2"),
                    categories::slug.eq("cat2::sub2"),
                ),
                (
                    categories::category.eq("Cat 3"),
                    categories::slug.eq("cat3"),
                ),
            ])
            .execute(&mut conn)
            .await
            .unwrap();

        let mut crate_idx = 0;
        let slugs = vec![
            ("cat1", 1),
            ("cat1::sub1", 2),
            ("cat1::sub2", 2),
            ("cat1::sub1::subsub1", 2),
        ];

        for (slug, count) in slugs {
            let cat_id: i32 = Category::by_slug(slug)
                .select(categories::id)
                .first::<i32>(&mut conn)
                .await
                .unwrap();
            for _ in 0..count {
                crate_idx += 1;
                let name = format!("k_{}", crate_idx);
                let k_id: i32 = diesel::insert_into(crates::table)
                    .values(crates::name.eq(name))
                    .returning(crates::id)
                    .get_result(&mut conn)
                    .await
                    .unwrap();
                diesel::insert_into(crates_categories::table)
                    .values((
                        crates_categories::crate_id.eq(k_id),
                        crates_categories::category_id.eq(cat_id),
                    ))
                    .execute(&mut conn)
                    .await
                    .unwrap();
            }
        }

        let cat: Category = Category::by_slug("cat1::sub1")
            .select(Category::as_select())
            .first(&mut conn)
            .await
            .unwrap();

        let subcats = cat.subcategories(&mut conn).await.unwrap();
        let parents = cat.parent_categories(&mut conn).await.unwrap();

        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0].slug, "cat1");
        assert_eq!(parents[0].crates_cnt, 7);
        assert_eq!(subcats.len(), 1);
        assert_eq!(subcats[0].slug, "cat1::sub1::subsub1");
    }

    #[tokio::test]
    async fn category_toplevel_unique_count_reproduction() {
        use self::categories;
        use crate::schema::crates;
        use crate::schema::crates_categories;

        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        // 1. Create a crate
        let crate_id: i32 = diesel::insert_into(crates::table)
            .values(crates::name.eq("crate_a"))
            .returning(crates::id)
            .get_result(&mut conn)
            .await
            .unwrap();

        let crate_id_2: i32 = diesel::insert_into(crates::table)
            .values(crates::name.eq("crate_b"))
            .returning(crates::id)
            .get_result(&mut conn)
            .await
            .unwrap();

        // 2. Setup categories
        diesel::insert_into(categories::table)
            .values(&vec![
                (
                    categories::category.eq("Science"),
                    categories::slug.eq("science"),
                ),
                (
                    categories::category.eq("Science::Chemistry"),
                    categories::slug.eq("science::chemistry"),
                ),
                (
                    categories::category.eq("Science::Physics"),
                    categories::slug.eq("science::physics"),
                ),
            ])
            .execute(&mut conn)
            .await
            .unwrap();

        let chem: Category = Category::by_slug("science::chemistry")
            .select(Category::as_select())
            .first(&mut conn)
            .await
            .unwrap();
        let phys: Category = Category::by_slug("science::physics")
            .select(Category::as_select())
            .first(&mut conn)
            .await
            .unwrap();

        // 3. Link the SAME crate to BOTH subcategories
        diesel::insert_into(crates_categories::table)
            .values(&vec![
                (
                    crates_categories::crate_id.eq(crate_id),
                    crates_categories::category_id.eq(chem.id),
                ),
                (
                    crates_categories::crate_id.eq(crate_id),
                    crates_categories::category_id.eq(phys.id),
                ),
                (
                    crates_categories::crate_id.eq(crate_id_2),
                    crates_categories::category_id.eq(phys.id),
                ),
            ])
            .execute(&mut conn)
            .await
            .unwrap();

        // EXECUTE: Fetch top-level categories
        let cats = Category::toplevel(&mut conn, "crates", 10, 0)
            .await
            .unwrap();

        let science = cats
            .iter()
            .find(|c| c.slug == "science")
            .expect("Parent category not found");

        // VERIFY: The parent count should be 2 (unique).
        assert_eq!(
            science.crates_cnt, 2,
            "Parent category should only count the unique crate once!"
        );
    }
}
