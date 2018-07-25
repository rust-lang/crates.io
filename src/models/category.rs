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
            category,
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
        use diesel::dsl::*;
        use diesel::select;

        let sort_sql = match sort {
            "crates" => "ORDER BY crates_cnt DESC",
            _ => "ORDER BY category ASC",
        };

        // Collect all the top-level categories and sum up the crates_cnt of
        // the crates in all subcategories
        select(sql::<categories::SqlType>(&format!(
            "c.id, c.category, c.slug, c.description,
                sum(c2.crates_cnt)::int as crates_cnt, c.created_at
             FROM categories as c
             INNER JOIN categories c2 ON split_part(c2.slug, '::', 1) = c.slug
             WHERE split_part(c.slug, '::', 1) = c.slug
             GROUP BY c.id
             {} LIMIT {} OFFSET {}",
            sort_sql, limit, offset
        ))).load(conn)
    }

    pub fn subcategories(&self, conn: &PgConnection) -> QueryResult<Vec<Category>> {
        use diesel::sql_types::Text;

        sql_query(
            "SELECT c.id, c.category, c.slug, c.description, \
             COALESCE (( \
             SELECT sum(c2.crates_cnt)::int \
             FROM categories as c2 \
             WHERE c2.slug = c.slug \
             OR c2.slug LIKE c.slug || '::%' \
             ), 0) as crates_cnt, c.created_at \
             FROM categories as c \
             WHERE c.category ILIKE $1 || '::%' \
             AND c.category NOT ILIKE $1 || '::%::%'",
        ).bind::<Text, _>(&self.category)
            .load(conn)
    }
}

/// Struct for inserting categories; only used in tests. Actual categories are inserted
/// in src/boot/categories.rs.
#[derive(Insertable, AsChangeset, Default, Debug)]
#[table_name = "categories"]
pub struct NewCategory<'a> {
    pub category: &'a str,
    pub slug: &'a str,
    pub description: &'a str,
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
        let conn = pg_connection();
        conn.batch_execute(
            "INSERT INTO categories (category, slug) VALUES
            ('Cat 2', 'cat2'), ('Cat 1', 'cat1'), ('Cat 1::sub', 'cat1::sub')
            ",
        ).unwrap();

        let categories = Category::toplevel(&conn, "", 10, 0)
            .unwrap()
            .into_iter()
            .map(|c| c.category)
            .collect::<Vec<_>>();
        let expected = vec!["Cat 1".to_string(), "Cat 2".to_string()];
        assert_eq!(expected, categories);
    }

    #[test]
    fn category_toplevel_orders_by_crates_cnt_when_sort_given() {
        let conn = pg_connection();
        conn.batch_execute(
            "INSERT INTO categories (category, slug, crates_cnt) VALUES
            ('Cat 1', 'cat1', 0), ('Cat 2', 'cat2', 2), ('Cat 3', 'cat3', 1)
            ",
        ).unwrap();

        let categories = Category::toplevel(&conn, "crates", 10, 0)
            .unwrap()
            .into_iter()
            .map(|c| c.category)
            .collect::<Vec<_>>();
        let expected = vec![
            "Cat 2".to_string(),
            "Cat 3".to_string(),
            "Cat 1".to_string(),
        ];
        assert_eq!(expected, categories);
    }

    #[test]
    fn category_toplevel_applies_limit_and_offset() {
        let conn = pg_connection();
        conn.batch_execute(
            "INSERT INTO categories (category, slug) VALUES
            ('Cat 1', 'cat1'), ('Cat 2', 'cat2')
            ",
        ).unwrap();

        let categories = Category::toplevel(&conn, "", 1, 0)
            .unwrap()
            .into_iter()
            .map(|c| c.category)
            .collect::<Vec<_>>();
        let expected = vec!["Cat 1".to_string()];
        assert_eq!(expected, categories);

        let categories = Category::toplevel(&conn, "", 1, 1)
            .unwrap()
            .into_iter()
            .map(|c| c.category)
            .collect::<Vec<_>>();
        let expected = vec!["Cat 2".to_string()];
        assert_eq!(expected, categories);
    }

    #[test]
    fn category_toplevel_includes_subcategories_in_crate_cnt() {
        let conn = pg_connection();
        conn.batch_execute(
            "INSERT INTO categories (category, slug, crates_cnt) VALUES
            ('Cat 1', 'cat1', 1), ('Cat 1::sub', 'cat1::sub', 2),
            ('Cat 2', 'cat2', 3), ('Cat 2::Sub 1', 'cat2::sub1', 4),
            ('Cat 2::Sub 2', 'cat2::sub2', 5), ('Cat 3', 'cat3', 6)
            ",
        ).unwrap();

        let categories = Category::toplevel(&conn, "crates", 10, 0)
            .unwrap()
            .into_iter()
            .map(|c| (c.category, c.crates_cnt))
            .collect::<Vec<_>>();
        let expected = vec![
            ("Cat 2".to_string(), 12),
            ("Cat 3".to_string(), 6),
            ("Cat 1".to_string(), 3),
        ];
        assert_eq!(expected, categories);
    }

    #[test]
    fn category_toplevel_applies_limit_and_offset_after_grouping() {
        let conn = pg_connection();
        conn.batch_execute(
            "INSERT INTO categories (category, slug, crates_cnt) VALUES
            ('Cat 1', 'cat1', 1), ('Cat 1::sub', 'cat1::sub', 2),
            ('Cat 2', 'cat2', 3), ('Cat 2::Sub 1', 'cat2::sub1', 4),
            ('Cat 2::Sub 2', 'cat2::sub2', 5), ('Cat 3', 'cat3', 6)
            ",
        ).unwrap();

        let categories = Category::toplevel(&conn, "crates", 2, 0)
            .unwrap()
            .into_iter()
            .map(|c| (c.category, c.crates_cnt))
            .collect::<Vec<_>>();
        let expected = vec![("Cat 2".to_string(), 12), ("Cat 3".to_string(), 6)];
        assert_eq!(expected, categories);

        let categories = Category::toplevel(&conn, "crates", 2, 1)
            .unwrap()
            .into_iter()
            .map(|c| (c.category, c.crates_cnt))
            .collect::<Vec<_>>();
        let expected = vec![("Cat 3".to_string(), 6), ("Cat 1".to_string(), 3)];
        assert_eq!(expected, categories);
    }
}
