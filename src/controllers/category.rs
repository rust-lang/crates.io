use super::prelude::*;

use crate::models::Category;
use crate::schema::categories;
use crate::views::{EncodableCategory, EncodableCategoryWithSubcategories};

/// Handles the `GET /categories` route.
pub fn index(req: &mut dyn Request) -> CargoResult<Response> {
    let conn = req.db_conn()?;
    let (offset, limit) = req.pagination(10, 100)?;
    let query = req.query();
    let sort = query.get("sort").map_or("alpha", String::as_str);

    let categories = Category::toplevel(&conn, sort, limit, offset)?;
    let categories = categories.into_iter().map(Category::encodable).collect();

    // Query for the total count of categories
    let total = Category::count_toplevel(&conn)?;

    #[derive(Serialize)]
    struct R {
        categories: Vec<EncodableCategory>,
        meta: Meta,
    }
    #[derive(Serialize)]
    struct Meta {
        total: i64,
    }

    Ok(req.json(&R {
        categories,
        meta: Meta { total },
    }))
}

/// Handles the `GET /categories/:category_id` route.
pub fn show(req: &mut dyn Request) -> CargoResult<Response> {
    let slug = &req.params()["category_id"];
    let conn = req.db_conn()?;
    let cat = Category::by_slug(slug).first::<Category>(&*conn)?;
    let subcats = cat
        .subcategories(&conn)?
        .into_iter()
        .map(Category::encodable)
        .collect();
    let parents = cat
        .parent_categories(&conn)?
        .into_iter()
        .map(Category::encodable)
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
        parent_categories: parents,
    };

    #[derive(Serialize)]
    struct R {
        category: EncodableCategoryWithSubcategories,
    }
    Ok(req.json(&R {
        category: cat_with_subcats,
    }))
}

/// Handles the `GET /category_slugs` route.
pub fn slugs(req: &mut dyn Request) -> CargoResult<Response> {
    let conn = req.db_conn()?;
    let slugs = categories::table
        .select((categories::slug, categories::slug, categories::description))
        .order(categories::slug)
        .load(&*conn)?;

    #[derive(Serialize, Queryable)]
    struct Slug {
        id: String,
        slug: String,
        description: String,
    }

    #[derive(Serialize)]
    struct R {
        category_slugs: Vec<Slug>,
    }
    Ok(req.json(&R {
        category_slugs: slugs,
    }))
}
