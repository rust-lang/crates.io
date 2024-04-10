use super::helpers::pagination::*;
use super::prelude::*;

use crate::models::Category;
use crate::schema::categories;
use crate::views::{EncodableCategory, EncodableCategoryWithSubcategories};

/// Handles the `GET /categories` route.
pub async fn index(app: AppState, req: Parts) -> AppResult<Json<Value>> {
    // FIXME: There are 69 categories, 47 top level. This isn't going to
    // grow by an OoM. We need a limit for /summary, but we don't need
    // to paginate this.
    let options = PaginationOptions::builder().gather(&req)?;

    let conn = app.db_read().await?;
    conn.interact(move |conn| {
        let query = req.query();
        let sort = query.get("sort").map_or("alpha", String::as_str);

        let offset = options.offset().unwrap_or_default();

        let categories = Category::toplevel(conn, sort, options.per_page, offset)?;
        let categories = categories
            .into_iter()
            .map(Category::into)
            .collect::<Vec<EncodableCategory>>();

        // Query for the total count of categories
        let total = Category::count_toplevel(conn)?;

        Ok(Json(json!({
            "categories": categories,
            "meta": { "total": total },
        })))
    })
    .await?
}

/// Handles the `GET /categories/:category_id` route.
pub async fn show(state: AppState, Path(slug): Path<String>) -> AppResult<Json<Value>> {
    let conn = state.db_read().await?;
    conn.interact(move |conn| {
        let cat: Category = Category::by_slug(&slug).first(conn)?;
        let subcats = cat
            .subcategories(conn)?
            .into_iter()
            .map(Category::into)
            .collect();
        let parents = cat
            .parent_categories(conn)?
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

        Ok(Json(json!({ "category": cat_with_subcats })))
    })
    .await?
}

/// Handles the `GET /category_slugs` route.
pub async fn slugs(state: AppState) -> AppResult<Json<Value>> {
    let conn = state.db_read().await?;
    conn.interact(move |conn| {
        let slugs: Vec<Slug> = categories::table
            .select((categories::slug, categories::slug, categories::description))
            .order(categories::slug)
            .load(conn)?;

        #[derive(Serialize, Queryable)]
        struct Slug {
            id: String,
            slug: String,
            description: String,
        }

        Ok(Json(json!({ "category_slugs": slugs })))
    })
    .await?
}
