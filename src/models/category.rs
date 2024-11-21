use chrono::NaiveDateTime;
use diesel::{
    delete, dsl, insert_into, sql_query, ExpressionMethods, QueryDsl, QueryResult,
    TextExpressionMethods,
};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection};

use crate::models::Crate;
use crate::schema::*;
use crate::util::diesel::Conn;

#[derive(Clone, Identifiable, Queryable, QueryableByName, Debug, Selectable)]
#[diesel(table_name = categories, check_for_backend(diesel::pg::Pg))]
pub struct Category {
    pub id: i32,
    pub category: String,
    pub slug: String,
    pub description: String,
    pub crates_cnt: i32,
    pub created_at: NaiveDateTime,
}

type WithSlug<'a> = diesel::dsl::Eq<categories::slug, crate::sql::lower<&'a str>>;

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
        categories::slug.eq(crate::sql::lower(slug))
    }

    #[dsl::auto_type(no_type_alias)]
    pub fn by_slug<'a>(slug: &'a str) -> _ {
        let filter: WithSlug<'a> = Self::with_slug(slug);
        categories::table.filter(filter)
    }

    pub async fn async_update_crate(
        conn: &mut AsyncPgConnection,
        crate_id: i32,
        slugs: &[&str],
    ) -> QueryResult<Vec<String>> {
        use diesel_async::RunQueryDsl;
        conn.transaction(|conn| {
            async move {
                let categories: Vec<Category> = categories::table
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

                delete(crates_categories::table)
                    .filter(crates_categories::crate_id.eq(crate_id))
                    .execute(conn)
                    .await?;

                insert_into(crates_categories::table)
                    .values(&crate_categories)
                    .execute(conn)
                    .await?;

                Ok(invalid_categories)
            }
            .scope_boxed()
        })
        .await
    }

    pub fn update_crate(
        conn: &mut impl Conn,
        crate_id: i32,
        slugs: &[&str],
    ) -> QueryResult<Vec<String>> {
        use diesel::RunQueryDsl;
        conn.transaction(|conn| {
            let categories: Vec<Category> = categories::table
                .filter(categories::slug.eq_any(slugs))
                .load(conn)?;
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

            delete(crates_categories::table)
                .filter(crates_categories::crate_id.eq(crate_id))
                .execute(conn)?;

            insert_into(crates_categories::table)
                .values(&crate_categories)
                .execute(conn)?;

            Ok(invalid_categories)
        })
    }

    pub async fn count_toplevel(conn: &mut AsyncPgConnection) -> QueryResult<i64> {
        use diesel_async::RunQueryDsl;
        categories::table
            .filter(categories::category.not_like("%::%"))
            .count()
            .get_result(conn)
            .await
    }

    pub async fn toplevel(
        conn: &mut AsyncPgConnection,
        sort: &str,
        limit: i64,
        offset: i64,
    ) -> QueryResult<Vec<Category>> {
        use diesel::sql_types::Int8;
        use diesel_async::RunQueryDsl;

        let sort_sql = match sort {
            "crates" => "ORDER BY crates_cnt DESC",
            _ => "ORDER BY category ASC",
        };

        // Collect all the top-level categories and sum up the crates_cnt of
        // the crates in all subcategories
        sql_query(format!(include_str!("toplevel.sql"), sort_sql))
            .bind::<Int8, _>(limit)
            .bind::<Int8, _>(offset)
            .load(conn)
            .await
    }

    pub async fn subcategories(&self, conn: &mut AsyncPgConnection) -> QueryResult<Vec<Category>> {
        use diesel::sql_types::Text;
        use diesel_async::RunQueryDsl;

        sql_query(include_str!("../subcategories.sql"))
            .bind::<Text, _>(&self.category)
            .load(conn)
            .await
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
        use diesel_async::RunQueryDsl;

        sql_query(include_str!("../parent_categories.sql"))
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

        insert_into(categories::table)
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

        let new_cat = |category, slug, crates_cnt| {
            (
                categories::category.eq(category),
                categories::slug.eq(slug),
                categories::crates_cnt.eq(crates_cnt),
            )
        };

        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        insert_into(categories::table)
            .values(&vec![
                new_cat("Cat 1", "cat1", 0),
                new_cat("Cat 2", "cat2", 2),
                new_cat("Cat 3", "cat3", 1),
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

        insert_into(categories::table)
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

        let new_cat = |category, slug, crates_cnt| {
            (
                categories::category.eq(category),
                categories::slug.eq(slug),
                categories::crates_cnt.eq(crates_cnt),
            )
        };

        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        insert_into(categories::table)
            .values(&vec![
                new_cat("Cat 1", "cat1", 1),
                new_cat("Cat 1::sub", "cat1::sub", 2),
                new_cat("Cat 2", "cat2", 3),
                new_cat("Cat 2::Sub 1", "cat2::sub1", 4),
                new_cat("Cat 2::Sub 2", "cat2::sub2", 5),
                new_cat("Cat 3", "cat3", 6),
            ])
            .execute(&mut conn)
            .await
            .unwrap();

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

        let new_cat = |category, slug, crates_cnt| {
            (
                categories::category.eq(category),
                categories::slug.eq(slug),
                categories::crates_cnt.eq(crates_cnt),
            )
        };

        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        insert_into(categories::table)
            .values(&vec![
                new_cat("Cat 1", "cat1", 1),
                new_cat("Cat 1::sub", "cat1::sub", 2),
                new_cat("Cat 2", "cat2", 3),
                new_cat("Cat 2::Sub 1", "cat2::sub1", 4),
                new_cat("Cat 2::Sub 2", "cat2::sub2", 5),
                new_cat("Cat 3", "cat3", 6),
            ])
            .execute(&mut conn)
            .await
            .unwrap();

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

        let new_cat = |category, slug, crates_cnt| {
            (
                categories::category.eq(category),
                categories::slug.eq(slug),
                categories::crates_cnt.eq(crates_cnt),
            )
        };

        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        insert_into(categories::table)
            .values(&vec![
                new_cat("Cat 1", "cat1", 1),
                new_cat("Cat 1::sub1", "cat1::sub1", 2),
                new_cat("Cat 1::sub2", "cat1::sub2", 2),
                new_cat("Cat 1::sub1::subsub1", "cat1::sub1::subsub1", 2),
                new_cat("Cat 2", "cat2", 3),
                new_cat("Cat 2::Sub 1", "cat2::sub1", 4),
                new_cat("Cat 2::Sub 2", "cat2::sub2", 5),
                new_cat("Cat 3", "cat3", 200),
            ])
            .execute(&mut conn)
            .await
            .unwrap();

        let cat: Category = Category::by_slug("cat1::sub1")
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
}
