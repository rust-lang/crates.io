use std::collections::HashSet;
use time::Timespec;

use conduit::{Request, Response};
use conduit_router::RequestParams;
use pg::GenericConnection;
use pg::rows::Row;

use {Model, Crate};
use db::RequestTransaction;
use util::{RequestUtils, CargoResult, ChainError};
use util::errors::NotFound;

#[derive(Clone)]
pub struct Category {
    pub id: i32,
    pub category: String,
    pub slug: String,
    pub description: String,
    pub created_at: Timespec,
    pub crates_cnt: i32,
}

#[derive(RustcEncodable, RustcDecodable)]
pub struct EncodableCategory {
    pub id: String,
    pub category: String,
    pub slug: String,
    pub description: String,
    pub created_at: String,
    pub crates_cnt: i32,
}

#[derive(RustcEncodable, RustcDecodable)]
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
    pub fn find_by_category(conn: &GenericConnection, name: &str)
                            -> CargoResult<Category> {
        let stmt = conn.prepare("SELECT * FROM categories \
                                      WHERE category = $1")?;
        let rows = stmt.query(&[&name])?;
        rows.iter().next()
                   .chain_error(|| NotFound)
                   .map(|row| Model::from_row(&row))
    }

    pub fn find_by_slug(conn: &GenericConnection, slug: &str)
                            -> CargoResult<Category> {
        let stmt = conn.prepare("SELECT * FROM categories \
                                      WHERE slug = LOWER($1)")?;
        let rows = stmt.query(&[&slug])?;
        rows.iter().next()
                   .chain_error(|| NotFound)
                   .map(|row| Model::from_row(&row))
    }

    pub fn encodable(self) -> EncodableCategory {
        let Category {
            id: _, crates_cnt, category, slug, description, created_at
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

    pub fn update_crate(conn: &GenericConnection,
                        krate: &Crate,
                        categories: &[String]) -> CargoResult<Vec<String>> {
        let old_categories = krate.categories(conn)?;
        let old_categories_ids: HashSet<_> = old_categories.iter().map(|cat| {
            cat.id
        }).collect();

        // If a new category specified is not in the database, filter
        // it out and don't add it. Return it to be able to warn about it.
        let mut invalid_categories = vec![];
        let new_categories: Vec<Category> = categories.iter().flat_map(|c| {
            match Category::find_by_slug(conn, &c) {
                Ok(cat) => Some(cat),
                Err(_) => {
                    invalid_categories.push(c.to_string());
                    None
                },
            }
        }).collect();

        let new_categories_ids: HashSet<_> = new_categories.iter().map(|cat| {
            cat.id
        }).collect();

        let to_rm: Vec<_> = old_categories_ids
                                .difference(&new_categories_ids)
                                .cloned()
                                .collect();
        let to_add: Vec<_> = new_categories_ids
                                .difference(&old_categories_ids)
                                .cloned()
                                .collect();

        if !to_rm.is_empty() {
            conn.execute("DELETE FROM crates_categories \
                                WHERE category_id = ANY($1) \
                                  AND crate_id = $2",
                         &[&to_rm, &krate.id])?;
        }

        if !to_add.is_empty() {
            let insert: Vec<_> = to_add.into_iter().map(|id| {
                format!("({}, {})", krate.id, id)
            }).collect();
            let insert = insert.join(", ");
            conn.execute(&format!("INSERT INTO crates_categories \
                                        (crate_id, category_id) VALUES {}",
                                  insert),
                         &[])?;
        }

        Ok(invalid_categories)
    }

    pub fn count_toplevel(conn: &GenericConnection) -> CargoResult<i64> {
        let sql = format!("\
            SELECT COUNT(*) \
            FROM {} \
            WHERE category NOT LIKE '%::%'",
            Model::table_name(None::<Self>
        ));
        let stmt = conn.prepare(&sql)?;
        let rows = stmt.query(&[])?;
        Ok(rows.iter().next().unwrap().get("count"))
    }

    pub fn toplevel(conn: &GenericConnection,
                    sort: &str,
                    limit: i64,
                    offset: i64) -> CargoResult<Vec<Category>> {

        let sort_sql = match sort {
            "crates" => "ORDER BY crates_cnt DESC",
            _ => "ORDER BY category ASC",
        };

        // Collect all the top-level categories and sum up the crates_cnt of
        // the crates in all subcategories
        let stmt = conn.prepare(&format!(
            "SELECT c.id, c.category, c.slug, c.description, c.created_at, \
                COALESCE (( \
                    SELECT sum(c2.crates_cnt)::int \
                    FROM categories as c2 \
                    WHERE c2.slug = c.slug \
                    OR c2.slug LIKE c.slug || '::%' \
                ), 0) as crates_cnt \
             FROM categories as c \
             WHERE c.category NOT LIKE '%::%' {} \
             LIMIT $1 OFFSET $2",
            sort_sql
        ))?;

        let categories: Vec<_> = stmt.query(&[&limit, &offset])?
            .iter()
            .map(|row| Model::from_row(&row))
            .collect();

        Ok(categories)
    }

    pub fn subcategories(&self, conn: &GenericConnection)
                                -> CargoResult<Vec<Category>> {
        let stmt = conn.prepare("\
            SELECT c.id, c.category, c.slug, c.description, c.created_at, \
            COALESCE (( \
                SELECT sum(c2.crates_cnt)::int \
                FROM categories as c2 \
                WHERE c2.slug = c.slug \
                OR c2.slug LIKE c.slug || '::%' \
            ), 0) as crates_cnt \
            FROM categories as c \
            WHERE c.category ILIKE $1 || '::%' \
            AND c.category NOT ILIKE $1 || '::%::%'")?;

        let rows = stmt.query(&[&self.category])?;
        Ok(rows.iter().map(|r| Model::from_row(&r)).collect())
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
    fn table_name(_: Option<Category>) -> &'static str { "categories" }
}

/// Handles the `GET /categories` route.
pub fn index(req: &mut Request) -> CargoResult<Response> {
    let conn = req.tx()?;
    let (offset, limit) = req.pagination(10, 100)?;
    let query = req.query();
    let sort = query.get("sort").map_or("alpha", String::as_str);

    let categories = Category::toplevel(conn, sort, limit, offset)?;
    let categories = categories.into_iter().map(Category::encodable).collect();

    // Query for the total count of categories
    let total = Category::count_toplevel(conn)?;

    #[derive(RustcEncodable)]
    struct R { categories: Vec<EncodableCategory>, meta: Meta }
    #[derive(RustcEncodable)]
    struct Meta { total: i64 }

    Ok(req.json(&R {
        categories: categories,
        meta: Meta { total: total },
    }))
}

/// Handles the `GET /categories/:category_id` route.
pub fn show(req: &mut Request) -> CargoResult<Response> {
    let slug = &req.params()["category_id"];
    let conn = req.tx()?;
    let cat = Category::find_by_slug(&*conn, &slug)?;
    let subcats = cat.subcategories(&*conn)?.into_iter().map(|s| {
        s.encodable()
    }).collect();
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
    struct R { category: EncodableCategoryWithSubcategories}
    Ok(req.json(&R { category: cat_with_subcats }))
}

/// Handles the `GET /category_slugs` route.
pub fn slugs(req: &mut Request) -> CargoResult<Response> {
    let conn = req.tx()?;
    let stmt = conn.prepare("SELECT slug FROM categories \
                                  ORDER BY slug")?;
    let rows = stmt.query(&[])?;

    #[derive(RustcEncodable)]
    struct Slug { id: String, slug: String }

    let slugs: Vec<Slug> = rows.iter().map(|r| {
        let slug: String = r.get("slug");
        Slug { id: slug.clone(), slug: slug }
    }).collect();

    #[derive(RustcEncodable)]
    struct R { category_slugs: Vec<Slug> }
    Ok(req.json(&R { category_slugs: slugs }))
}
