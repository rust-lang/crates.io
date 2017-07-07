use std::collections::HashSet;
use time::Timespec;

use conduit::{Request, Response};
use conduit_router::RequestParams;
use diesel::*;
use diesel::pg::PgConnection;
use pg::GenericConnection;
use pg::rows::Row;

use db::RequestTransaction;
use schema::*;
use util::errors::NotFound;
use util::{RequestUtils, CargoResult, ChainError};
use {Model, Crate};

#[derive(Clone, Identifiable, Queryable, Debug)]
#[table_name = "categories"]
pub struct Category {
    pub id: i32,
    pub category: String,
    pub slug: String,
    pub description: String,
    pub crates_cnt: i32,
    pub created_at: Timespec,
}

#[derive(Associations, Insertable, Identifiable, Debug)]
#[belongs_to(Category)]
#[belongs_to(Crate)]
#[table_name = "crates_categories"]
#[primary_key(crate_id, category_id)]
pub struct CrateCategory {
    crate_id: i32,
    category_id: i32,
}

#[derive(RustcEncodable, RustcDecodable, Debug)]
pub struct EncodableCategory {
    pub id: String,
    pub category: String,
    pub slug: String,
    pub description: String,
    pub created_at: String,
    pub crates_cnt: i32,
}

#[derive(RustcEncodable, RustcDecodable, Debug)]
pub struct EncodableCategoryWithSubcategories {
    pub id: String,
    pub category: String,
    pub slug: String,
    pub description: String,
    pub created_at: String,
    pub crates_cnt: i32,
    pub subcategories: Vec<EncodableCategory>,
}

impl Category {
    pub fn find_by_category(conn: &GenericConnection, name: &str) -> CargoResult<Category> {
        let stmt = conn.prepare(
            "SELECT * FROM categories \
             WHERE category = $1",
        )?;
        let rows = stmt.query(&[&name])?;
        rows.iter().next().chain_error(|| NotFound).map(|row| {
            Model::from_row(&row)
        })
    }

    pub fn find_by_slug(conn: &GenericConnection, slug: &str) -> CargoResult<Category> {
        let stmt = conn.prepare(
            "SELECT * FROM categories \
             WHERE slug = LOWER($1)",
        )?;
        let rows = stmt.query(&[&slug])?;
        rows.iter().next().chain_error(|| NotFound).map(|row| {
            Model::from_row(&row)
        })
    }

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
            created_at: ::encode_time(created_at),
            crates_cnt: crates_cnt,
            category: category,
        }
    }

    pub fn update_crate<'a>(
        conn: &PgConnection,
        krate: &Crate,
        slugs: &[&'a str],
    ) -> QueryResult<Vec<&'a str>> {
        use diesel::expression::dsl::any;

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
                .map(|c| {
                    CrateCategory {
                        category_id: c.id,
                        crate_id: krate.id,
                    }
                })
                .collect::<Vec<_>>();

            delete(CrateCategory::belonging_to(krate)).execute(conn)?;
            insert(&crate_categories)
                .into(crates_categories::table)
                .execute(conn)?;
            Ok(invalid_categories)
        })
    }

    pub fn update_crate_old(
        conn: &GenericConnection,
        krate: &Crate,
        categories: &[String],
    ) -> CargoResult<Vec<String>> {
        let old_categories = krate.categories(conn)?;
        let old_categories_ids: HashSet<_> = old_categories.iter().map(|cat| cat.id).collect();

        // If a new category specified is not in the database, filter
        // it out and don't add it. Return it to be able to warn about it.
        let mut invalid_categories = vec![];
        let new_categories: Vec<Category> = categories
            .iter()
            .flat_map(|c| match Category::find_by_slug(conn, c) {
                Ok(cat) => Some(cat),
                Err(_) => {
                    invalid_categories.push(c.to_string());
                    None
                }
            })
            .collect();

        let new_categories_ids: HashSet<_> = new_categories.iter().map(|cat| cat.id).collect();

        let to_rm: Vec<_> = old_categories_ids
            .difference(&new_categories_ids)
            .cloned()
            .collect();
        let to_add: Vec<_> = new_categories_ids
            .difference(&old_categories_ids)
            .cloned()
            .collect();

        if !to_rm.is_empty() {
            conn.execute(
                "DELETE FROM crates_categories \
                 WHERE category_id = ANY($1) \
                 AND crate_id = $2",
                &[&to_rm, &krate.id],
            )?;
        }

        if !to_add.is_empty() {
            let insert: Vec<_> = to_add
                .into_iter()
                .map(|id| format!("({}, {})", krate.id, id))
                .collect();
            let insert = insert.join(", ");
            conn.execute(
                &format!(
                    "INSERT INTO crates_categories \
                     (crate_id, category_id) VALUES {}",
                    insert
                ),
                &[],
            )?;
        }

        Ok(invalid_categories)
    }

    pub fn count_toplevel(conn: &GenericConnection) -> CargoResult<i64> {
        let sql = format!(
            "\
             SELECT COUNT(*) \
             FROM {} \
             WHERE category NOT LIKE '%::%'",
            Model::table_name(None::<Self>)
        );
        let stmt = conn.prepare(&sql)?;
        let rows = stmt.query(&[])?;
        Ok(rows.iter().next().unwrap().get("count"))
    }

    pub fn toplevel(
        conn: &PgConnection,
        sort: &str,
        limit: i64,
        offset: i64,
    ) -> QueryResult<Vec<Category>> {
        use diesel::select;
        use diesel::expression::dsl::*;

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
            sort_sql,
            limit,
            offset
        ))).load(conn)
    }

    pub fn toplevel_old(
        conn: &GenericConnection,
        sort: &str,
        limit: i64,
        offset: i64,
    ) -> CargoResult<Vec<Category>> {

        let sort_sql = match sort {
            "crates" => "ORDER BY crates_cnt DESC",
            _ => "ORDER BY category ASC",
        };

        // Collect all the top-level categories and sum up the crates_cnt of
        // the crates in all subcategories
        let stmt = conn.prepare(&format!(
            "SELECT c.id, c.category, c.slug, c.description, c.created_at,
                sum(c2.crates_cnt)::int as crates_cnt
             FROM categories as c
             INNER JOIN categories c2 ON split_part(c2.slug, '::', 1) = c.slug
             WHERE split_part(c.slug, '::', 1) = c.slug
             GROUP BY c.id
             {} LIMIT $1 OFFSET $2",
            sort_sql
        ))?;

        let categories: Vec<_> = stmt.query(&[&limit, &offset])?
            .iter()
            .map(|row| Model::from_row(&row))
            .collect();

        Ok(categories)
    }

    pub fn subcategories(&self, conn: &GenericConnection) -> CargoResult<Vec<Category>> {
        let stmt = conn.prepare(
            "\
             SELECT c.id, c.category, c.slug, c.description, c.created_at, \
             COALESCE (( \
             SELECT sum(c2.crates_cnt)::int \
             FROM categories as c2 \
             WHERE c2.slug = c.slug \
             OR c2.slug LIKE c.slug || '::%' \
             ), 0) as crates_cnt \
             FROM categories as c \
             WHERE c.category ILIKE $1 || '::%' \
             AND c.category NOT ILIKE $1 || '::%::%'",
        )?;

        let rows = stmt.query(&[&self.category])?;
        Ok(rows.iter().map(|r| Model::from_row(&r)).collect())
    }
}

#[derive(Insertable, Default, Debug)]
#[table_name = "categories"]
pub struct NewCategory<'a> {
    pub category: &'a str,
    pub slug: &'a str,
}

impl<'a> NewCategory<'a> {
    pub fn find_or_create(&self, conn: &PgConnection) -> QueryResult<Category> {
        use schema::categories::dsl::*;
        use diesel::pg::upsert::*;

        let maybe_inserted = insert(&self.on_conflict_do_nothing())
            .into(categories)
            .get_result(conn)
            .optional()?;

        if let Some(c) = maybe_inserted {
            return Ok(c);
        }

        categories.filter(slug.eq(self.slug)).first(conn)
    }
}

impl Model for Category {
    fn from_row(row: &Row) -> Category {
        Category {
            id: row.get("id"),
            created_at: row.get("created_at"),
            crates_cnt: row.get("crates_cnt"),
            category: row.get("category"),
            slug: row.get("slug"),
            description: row.get("description"),
        }
    }
    fn table_name(_: Option<Category>) -> &'static str {
        "categories"
    }
}

/// Handles the `GET /categories` route.
pub fn index(req: &mut Request) -> CargoResult<Response> {
    let conn = req.tx()?;
    let (offset, limit) = req.pagination(10, 100)?;
    let query = req.query();
    let sort = query.get("sort").map_or("alpha", String::as_str);

    let categories = Category::toplevel_old(conn, sort, limit, offset)?;
    let categories = categories.into_iter().map(Category::encodable).collect();

    // Query for the total count of categories
    let total = Category::count_toplevel(conn)?;

    #[derive(RustcEncodable)]
    struct R {
        categories: Vec<EncodableCategory>,
        meta: Meta,
    }
    #[derive(RustcEncodable)]
    struct Meta {
        total: i64,
    }

    Ok(req.json(&R {
        categories: categories,
        meta: Meta { total: total },
    }))
}

/// Handles the `GET /categories/:category_id` route.
pub fn show(req: &mut Request) -> CargoResult<Response> {
    let slug = &req.params()["category_id"];
    let conn = req.tx()?;
    let cat = Category::find_by_slug(conn, slug)?;
    let subcats = cat.subcategories(conn)?
        .into_iter()
        .map(|s| s.encodable())
        .collect();
    let cat = cat.encodable();
    let cat_with_subcats = EncodableCategoryWithSubcategories {
        id: cat.id,
        category: cat.category,
        slug: cat.slug,
        description: cat.description,
        created_at: cat.created_at,
        crates_cnt: cat.crates_cnt,
        subcategories: subcats,
    };

    #[derive(RustcEncodable)]
    struct R {
        category: EncodableCategoryWithSubcategories,
    }
    Ok(req.json(&R { category: cat_with_subcats }))
}

/// Handles the `GET /category_slugs` route.
pub fn slugs(req: &mut Request) -> CargoResult<Response> {
    let conn = req.tx()?;
    let stmt = conn.prepare(
        "SELECT slug FROM categories \
         ORDER BY slug",
    )?;
    let rows = stmt.query(&[])?;

    #[derive(RustcEncodable)]
    struct Slug {
        id: String,
        slug: String,
    }

    let slugs: Vec<Slug> = rows.iter()
        .map(|r| {
            let slug: String = r.get("slug");
            Slug {
                id: slug.clone(),
                slug: slug,
            }
        })
        .collect();

    #[derive(RustcEncodable)]
    struct R {
        category_slugs: Vec<Slug>,
    }
    Ok(req.json(&R { category_slugs: slugs }))
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
