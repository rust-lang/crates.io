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
    pub created_at: Timespec,
    pub crates_cnt: i32,
}

#[derive(RustcEncodable, RustcDecodable)]
pub struct EncodableCategory {
    pub id: String,
    pub category: String,
    pub created_at: String,
    pub crates_cnt: i32,
}

impl Category {
    pub fn find_by_category(conn: &GenericConnection, name: &str)
                            -> CargoResult<Option<Category>> {
        let stmt = try!(conn.prepare("SELECT * FROM categories \
                                      WHERE category = $1"));
        let rows = try!(stmt.query(&[&name]));
        Ok(rows.iter().next().map(|r| Model::from_row(&r)))
    }

    pub fn find_all_by_category(conn: &GenericConnection, names: &[String])
                                  -> CargoResult<Vec<Category>> {
        let stmt = try!(conn.prepare("SELECT * FROM categories \
                                      WHERE category = ANY($1)"));
        let rows = try!(stmt.query(&[&names]));
        Ok(rows.iter().map(|r| Model::from_row(&r)).collect())
    }

    pub fn encodable(self) -> EncodableCategory {
        let Category { id: _, crates_cnt, category, created_at } = self;
        EncodableCategory {
            id: category.clone(),
            created_at: ::encode_time(created_at),
            crates_cnt: crates_cnt,
            category: category,
        }
    }

    pub fn update_crate(conn: &GenericConnection,
                        krate: &Crate,
                        categories: &[String]) -> CargoResult<()> {
        let old_categories = try!(krate.categories(conn));
        let old_categories_ids: HashSet<_> = old_categories.iter().map(|cat| {
            cat.id
        }).collect();
        // If a new category specified is not in the database, filter
        // it out and don't add it.
        let new_categories = try!(
            Category::find_all_by_category(conn, categories)
        );
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

        if to_rm.len() > 0 {
            try!(conn.execute("DELETE FROM crates_categories \
                                WHERE category_id = ANY($1) \
                                  AND crate_id = $2",
                              &[&to_rm, &krate.id]));
        }

        if !to_add.is_empty() {
            let insert: Vec<_> = to_add.into_iter().map(|id| {
                format!("({}, {})", krate.id, id)
            }).collect();
            let insert = insert.join(", ");
            try!(conn.execute(&format!("INSERT INTO crates_categories \
                                        (crate_id, category_id) VALUES {}",
                                       insert),
                              &[]));
        }

        Ok(())
    }
}

impl Model for Category {
    fn from_row(row: &Row) -> Category {
        Category {
            id: row.get("id"),
            created_at: row.get("created_at"),
            crates_cnt: row.get("crates_cnt"),
            category: row.get("category"),
        }
    }
    fn table_name(_: Option<Category>) -> &'static str { "categories" }
}

/// Handles the `GET /categories` route.
pub fn index(req: &mut Request) -> CargoResult<Response> {
    let conn = try!(req.tx());
    let (offset, limit) = try!(req.pagination(10, 100));
    let query = req.query();
    let sort = query.get("sort").map(|s| &s[..]).unwrap_or("alpha");
    let sort_sql = match sort {
        "crates" => "ORDER BY crates_cnt DESC",
        _ => "ORDER BY category ASC",
    };

    // Collect all the categories
    let stmt = try!(conn.prepare(&format!("SELECT * FROM categories {} \
                                           LIMIT $1 OFFSET $2",
                                          sort_sql)));

    let categories: Vec<_> = try!(stmt.query(&[&limit, &offset]))
        .iter()
        .map(|row| {
            let category: Category = Model::from_row(&row);
            category.encodable()
        })
        .collect();

    // Query for the total count of categories
    let total = try!(Category::count(conn));

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
    let name = &req.params()["category_id"];
    let conn = try!(req.tx());
    let cat = try!(Category::find_by_category(&*conn, &name));
    let cat = try!(cat.chain_error(|| NotFound));

    #[derive(RustcEncodable)]
    struct R { category: EncodableCategory }
    Ok(req.json(&R { category: cat.encodable() }))
}
