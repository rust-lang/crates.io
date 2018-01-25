use chrono::NaiveDateTime;
use diesel::*;

use models::Crate;
use schema::*;
use views::EncodableCategory;

#[derive(Clone, Identifiable, Queryable, QueryableByName, Debug)]
#[table_name = "categories"]
pub struct Category {
    pub id: i32,
    pub category: String,
    pub slug: String,
    pub description: String,
    pub crates_cnt: i32,
    pub created_at: NaiveDateTime,
}

#[derive(Associations, Insertable, Identifiable, Debug, Clone, Copy)]
#[belongs_to(Category)]
#[belongs_to(Crate)]
#[table_name = "crates_categories"]
#[primary_key(crate_id, category_id)]
pub struct CrateCategory {
    crate_id: i32,
    category_id: i32,
}

impl Category {
    pub fn encodable(self) -> EncodableCategory {
        let Category {
            crates_cnt,
            category,
            slug,
            description,
            created_at,
            ..
        } = self;
        EncodableCategory {
            id: slug.clone(),
            slug: slug.clone(),
            description: description.clone(),
            created_at,
            crates_cnt,
            category: category.rsplit("::").collect::<Vec<_>>()[0].to_string(),
        }
    }

    pub fn update_crate<'a>(
        conn: &PgConnection,
        krate: &Crate,
        slugs: &[&'a str],
    ) -> QueryResult<Vec<&'a str>> {
        use diesel::dsl::any;

        conn.transaction(|| {
            let categories = categories::table
                .filter(categories::slug.eq(any(slugs)))
                .load::<Category>(conn)?;
            let invalid_categories = slugs
                .iter()
                .cloned()
                .filter(|s| !categories.iter().any(|c| c.slug == *s))
                .collect();
            let crate_categories = categories
                .iter()
                .map(|c| CrateCategory {
                    category_id: c.id,
                    crate_id: krate.id,
                })
                .collect::<Vec<_>>();

            delete(CrateCategory::belonging_to(krate)).execute(conn)?;
            insert_into(crates_categories::table)
                .values(&crate_categories)
                .execute(conn)?;
            Ok(invalid_categories)
        })
    }

    pub fn count_toplevel(conn: &PgConnection) -> QueryResult<i64> {
        use self::categories::dsl::*;

        categories
            .filter(category.not_like("%::%"))
            .count()
            .get_result(conn)
    }

    pub fn toplevel(
        conn: &PgConnection,
        sort: &str,
        limit: i64,
        offset: i64,
    ) -> QueryResult<Vec<Category>> {
        let sort_sql = match sort {
            "crates" => "ORDER BY crates_cnt DESC",
            _ => "ORDER BY category ASC",
        };

        // Collect all the top-level categories and sum up the crates_cnt of
        // the crates in all subcategories
        sql_query(format!(
            "SELECT c.id, c.category, c.slug, c.description, \
             sum(c2.crates_cnt)::int as crates_cnt, c.created_at \
             FROM categories as c \
             INNER JOIN categories c2 ON split_part(c2.slug, '::', 1) = c.slug \
             WHERE split_part(c.slug, '::', 1) = c.slug \
             GROUP BY c.id \
             {} LIMIT {} OFFSET {}",
            sort_sql, limit, offset
        )).load(conn)
    }

    pub fn subcategories(&self, conn: &PgConnection) -> QueryResult<Vec<Category>> {
        use diesel::sql_types::Text;

        sql_query(include_str!("../subcategories.sql"))
            .bind::<Text, _>(&self.category)
            .load(conn)
    }

    /// Gathers the parent categories from the top-level Category to the direct parent of this Category.
    /// Returns categories as a Vector in order of traversal, not including this Category.
    /// The intention is to be able to have slugs or parent categories arrayed in order, to
    /// offer the frontend, for examples, slugs to create links to each parent category in turn.
    pub fn parent_categories(&self, conn: &PgConnection) -> QueryResult<Vec<Category>> {
        use diesel::expression::dsl::*;
        use diesel::types::Text;

        sql_query(include_str!("../parent_categories.sql"))
            .bind::<Text, _>(&self.slug)
            .load(conn)
    }
}

#[derive(Insertable, AsChangeset, Default, Debug)]
#[table_name = "categories"]
pub struct NewCategory<'a> {
    pub category: &'a str,
    pub slug: &'a str,
}

impl<'a> NewCategory<'a> {
    /// Inserts the category into the database, or updates an existing one.
    pub fn create_or_update(&self, conn: &PgConnection) -> QueryResult<Category> {
        use schema::categories::dsl::*;

        insert_into(categories)
            .values(self)
            .on_conflict(slug)
            .do_update()
            .set(self)
            .get_result(conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::connection::SimpleConnection;
    use dotenv::dotenv;
    use std::env;

    fn pg_connection() -> PgConnection {
        let _ = dotenv();
        let database_url =
            env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set to run tests");
        let conn = PgConnection::establish(&database_url).unwrap();
        // These tests deadlock if run concurrently
        conn.batch_execute("BEGIN; LOCK categories IN ACCESS EXCLUSIVE MODE")
            .unwrap();
        conn
    }

    #[test]
    fn category_toplevel_excludes_subcategories() {
        use self::categories::dsl::*;
        let conn = pg_connection();
        insert_into(categories)
            .values(&vec![
                (category.eq("Cat 2"), slug.eq("cat2")),
                (category.eq("Cat 1"), slug.eq("cat1")),
                (category.eq("Cat 1::sub"), slug.eq("cat1::sub")),
            ])
            .execute(&conn)
            .unwrap();

        let cats = Category::toplevel(&conn, "", 10, 0)
            .unwrap()
            .into_iter()
            .map(|c| c.category)
            .collect::<Vec<_>>();
        let expected = vec!["Cat 1".to_string(), "Cat 2".to_string()];
        assert_eq!(expected, cats);
    }

    #[test]
    fn category_toplevel_orders_by_crates_cnt_when_sort_given() {
        use self::categories::dsl::*;
        let conn = pg_connection();
        insert_into(categories)
            .values(&vec![
                (category.eq("Cat 1"), slug.eq("cat1"), crates_cnt.eq(0)),
                (category.eq("Cat 2"), slug.eq("cat2"), crates_cnt.eq(2)),
                (category.eq("Cat 3"), slug.eq("cat3"), crates_cnt.eq(1)),
            ])
            .execute(&conn)
            .unwrap();

        let cats = Category::toplevel(&conn, "crates", 10, 0)
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

    #[test]
    fn category_toplevel_applies_limit_and_offset() {
        use self::categories::dsl::*;
        let conn = pg_connection();
        insert_into(categories)
            .values(&vec![
                (category.eq("Cat 1"), slug.eq("cat1")),
                (category.eq("Cat 2"), slug.eq("cat2")),
            ])
            .execute(&conn)
            .unwrap();

        let cats = Category::toplevel(&conn, "", 1, 0)
            .unwrap()
            .into_iter()
            .map(|c| c.category)
            .collect::<Vec<_>>();
        let expected = vec!["Cat 1".to_string()];
        assert_eq!(expected, cats);

        let cats = Category::toplevel(&conn, "", 1, 1)
            .unwrap()
            .into_iter()
            .map(|c| c.category)
            .collect::<Vec<_>>();
        let expected = vec!["Cat 2".to_string()];
        assert_eq!(expected, cats);
    }

    #[test]
    fn category_toplevel_includes_subcategories_in_crate_cnt() {
        use self::categories::dsl::*;
        let conn = pg_connection();
        insert_into(categories)
            .values(&vec![
                (category.eq("Cat 1"), slug.eq("cat1"), crates_cnt.eq(1)),
                (
                    category.eq("Cat 1::sub"),
                    slug.eq("cat1::sub"),
                    crates_cnt.eq(2),
                ),
                (category.eq("Cat 2"), slug.eq("cat2"), crates_cnt.eq(3)),
                (
                    category.eq("Cat 2::Sub 1"),
                    slug.eq("cat2::sub1"),
                    crates_cnt.eq(4),
                ),
                (
                    category.eq("Cat 2::Sub 2"),
                    slug.eq("cat2::sub2"),
                    crates_cnt.eq(5),
                ),
                (category.eq("Cat 3"), slug.eq("cat3"), crates_cnt.eq(6)),
            ])
            .execute(&conn)
            .unwrap();

        let cats = Category::toplevel(&conn, "crates", 10, 0)
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

    #[test]
    fn category_toplevel_applies_limit_and_offset_after_grouping() {
        use self::categories::dsl::*;
        let conn = pg_connection();
        insert_into(categories)
            .values(&vec![
                (category.eq("Cat 1"), slug.eq("cat1"), crates_cnt.eq(1)),
                (
                    category.eq("Cat 1::sub"),
                    slug.eq("cat1::sub"),
                    crates_cnt.eq(2),
                ),
                (category.eq("Cat 2"), slug.eq("cat2"), crates_cnt.eq(3)),
                (
                    category.eq("Cat 2::Sub 1"),
                    slug.eq("cat2::sub1"),
                    crates_cnt.eq(4),
                ),
                (
                    category.eq("Cat 2::Sub 2"),
                    slug.eq("cat2::sub2"),
                    crates_cnt.eq(5),
                ),
                (category.eq("Cat 3"), slug.eq("cat3"), crates_cnt.eq(6)),
            ])
            .execute(&conn)
            .unwrap();

        let cats = Category::toplevel(&conn, "crates", 2, 0)
            .unwrap()
            .into_iter()
            .map(|c| (c.category, c.crates_cnt))
            .collect::<Vec<_>>();
        let expected = vec![("Cat 2".to_string(), 12), ("Cat 3".to_string(), 6)];
        assert_eq!(expected, cats);

        let cats = Category::toplevel(&conn, "crates", 2, 1)
            .unwrap()
            .into_iter()
            .map(|c| (c.category, c.crates_cnt))
            .collect::<Vec<_>>();
        let expected = vec![("Cat 3".to_string(), 6), ("Cat 1".to_string(), 3)];
        assert_eq!(expected, cats);
    }

    #[test]
    fn category_parent_categories_includes_path_to_node_with_count() {
        use self::categories::dsl::*;
        let conn = pg_connection();
        insert_into(categories)
            .values(&vec![
                (category.eq("Cat 1"), slug.eq("cat1"), crates_cnt.eq(1)),
                (
                    category.eq("Cat 1::sub1"),
                    slug.eq("cat1::sub1"),
                    crates_cnt.eq(2),
                ),
                (
                    category.eq("Cat 1::sub2"),
                    slug.eq("cat1::sub2"),
                    crates_cnt.eq(2),
                ),
                (
                    category.eq("Cat 1::sub1::subsub1"),
                    slug.eq("cat1::sub1::subsub1"),
                    crates_cnt.eq(2),
                ),
                (category.eq("Cat 2"), slug.eq("cat2"), crates_cnt.eq(3)),
                (
                    category.eq("Cat 2::Sub 1"),
                    slug.eq("cat2::sub1"),
                    crates_cnt.eq(4),
                ),
                (
                    category.eq("Cat 2::Sub 2"),
                    slug.eq("cat2::sub2"),
                    crates_cnt.eq(5),
                ),
                (category.eq("Cat 3"), slug.eq("cat3"), crates_cnt.eq(200)),
            ])
            .execute(&conn)
            .unwrap();

        let cat = categories
            .filter(slug.eq("cat1::sub1"))
            .first::<Category>(&conn)
            .unwrap();
        let subcats = cat.subcategories(&conn).unwrap();
        let parents = cat.parent_categories(&conn).unwrap();

        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0].slug, "cat1");
        assert_eq!(parents[0].crates_cnt, 7);
        assert_eq!(subcats.len(), 1);
        assert_eq!(subcats[0].slug, "cat1::sub1::subsub1");
    }
}
