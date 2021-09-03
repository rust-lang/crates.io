use super::helpers::pagination::*;
use super::prelude::*;

use crate::models::Category;
use crate::schema::categories;
use crate::views::{EncodableCategory, EncodableCategoryWithSubcategories};

/// Handles the `GET /categories` route.
pub fn index(req: &mut dyn RequestExt) -> EndpointResult {
    let query = req.query();
    // FIXME: There are 69 categories, 47 top level. This isn't going to
    // grow by an OoM. We need a limit for /summary, but we don't need
    // to paginate this.
    let options = PaginationOptions::builder().gather(req)?;
    let offset = options.offset().unwrap_or_default();
    let sort = query.get("sort").map_or("alpha", String::as_str);

    let conn = req.db_read_only()?;
    let categories =
        Category::toplevel(&conn, sort, i64::from(options.per_page), i64::from(offset))?;
    let categories = categories
        .into_iter()
        .map(Category::into)
        .collect::<Vec<EncodableCategory>>();

    // Query for the total count of categories
    let total = Category::count_toplevel(&conn)?;

    Ok(req.json(&json!({
        "categories": categories,
        "meta": { "total": total },
    })))
}

/// Handles the `GET /categories/:category_id` route.
pub fn show(req: &mut dyn RequestExt) -> EndpointResult {
    let slug = &req.params()["category_id"];
    let conn = req.db_read_only()?;
    let cat: Category = Category::by_slug(slug).first(&*conn)?;
    let subcats = cat
        .subcategories(&conn)?
        .into_iter()
        .map(Category::into)
        .collect();
    let parents = cat
        .parent_categories(&conn)?
        .into_iter()
        .map(Category::into)
        .collect();

    let cat = EncodableCategory::from(cat);
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

    Ok(req.json(&json!({ "category": cat_with_subcats })))
}

/// Handles the `GET /category_slugs` route.
pub fn slugs(req: &mut dyn RequestExt) -> EndpointResult {
    let conn = req.db_read_only()?;
    let slugs: Vec<Slug> = categories::table
        .select((categories::slug, categories::slug, categories::description))
        .order(categories::slug)
        .load(&*conn)?;

    #[derive(Serialize, Queryable)]
    struct Slug {
        id: String,
        slug: String,
        description: String,
    }

    Ok(req.json(&json!({ "category_slugs": slugs })))
}
