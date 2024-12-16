use super::helpers::pagination::*;
use crate::app::AppState;
use crate::models::Category;
use crate::schema::categories;
use crate::util::errors::AppResult;
use crate::util::RequestUtils;
use crate::views::{EncodableCategory, EncodableCategoryWithSubcategories};
use axum::extract::Path;
use axum_extra::json;
use axum_extra::response::ErasedJson;
use diesel::QueryDsl;
use diesel_async::RunQueryDsl;
use http::request::Parts;

/// List all categories.
#[utoipa::path(
    get,
    path = "/api/v1/categories",
    tag = "categories",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn list_categories(app: AppState, req: Parts) -> AppResult<ErasedJson> {
    // FIXME: There are 69 categories, 47 top level. This isn't going to
    // grow by an OoM. We need a limit for /summary, but we don't need
    // to paginate this.
    let options = PaginationOptions::builder().gather(&req)?;

    let mut conn = app.db_read().await?;

    let query = req.query();
    let sort = query.get("sort").map_or("alpha", String::as_str);

    let offset = options.offset().unwrap_or_default();

    let categories = Category::toplevel(&mut conn, sort, options.per_page, offset).await?;
    let categories = categories
        .into_iter()
        .map(Category::into)
        .collect::<Vec<EncodableCategory>>();

    // Query for the total count of categories
    let total = Category::count_toplevel(&mut conn).await?;

    Ok(json!({
        "categories": categories,
        "meta": { "total": total },
    }))
}

/// Get category metadata.
#[utoipa::path(
    get,
    path = "/api/v1/categories/{category}",
    params(
        ("category" = String, Path, description = "Name of the category"),
    ),
    tag = "categories",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn find_category(state: AppState, Path(slug): Path<String>) -> AppResult<ErasedJson> {
    let mut conn = state.db_read().await?;

    let cat: Category = Category::by_slug(&slug).first(&mut conn).await?;
    let subcats = cat
        .subcategories(&mut conn)
        .await?
        .into_iter()
        .map(Category::into)
        .collect();
    let parents = cat
        .parent_categories(&mut conn)
        .await?
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

    Ok(json!({ "category": cat_with_subcats }))
}

/// List all available category slugs.
#[utoipa::path(
    get,
    path = "/api/v1/category_slugs",
    tag = "categories",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn list_category_slugs(state: AppState) -> AppResult<ErasedJson> {
    let mut conn = state.db_read().await?;

    let slugs: Vec<Slug> = categories::table
        .select((categories::slug, categories::slug, categories::description))
        .order(categories::slug)
        .load(&mut conn)
        .await?;

    #[derive(Serialize, Queryable)]
    struct Slug {
        id: String,
        slug: String,
        description: String,
    }

    Ok(json!({ "category_slugs": slugs }))
}
