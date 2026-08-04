use super::helpers::pagination::*;
use crate::app::AppState;
use crate::models::Category;
use crate::schema::categories;
use crate::util::errors::{AppResult, not_found};
use crate::views::EncodableCategory;
use axum::Json;
use axum::extract::{FromRequestParts, Path, Query};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::request::Parts;
use serde::{Deserialize, Serialize};

/// Query parameters for listing categories.
#[derive(Debug, Deserialize, FromRequestParts, utoipa::IntoParams)]
#[from_request(via(Query))]
#[into_params(parameter_in = Query)]
pub struct CategoryListQueryParams {
    /// The sort order of the categories.
    ///
    /// Valid values: `alpha`, and `crates`.
    ///
    /// Defaults to `alpha`.
    sort: Option<String>,
}

/// Response returned when listing categories.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CategoryListResponse {
    /// The list of categories.
    pub categories: Vec<EncodableCategory>,

    #[schema(inline)]
    pub meta: CategoryListMeta,
}

/// Pagination metadata for a category list response.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CategoryListMeta {
    /// The total number of categories.
    #[schema(example = 123)]
    pub total: i64,
}

/// List all categories.
#[utoipa::path(
    get,
    path = "/api/v1/categories",
    params(CategoryListQueryParams, PaginationQueryParams),
    tag = "categories",
    responses(
        (status = 200, description = "Successful Response", body = inline(CategoryListResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn list_categories(
    app: AppState,
    params: CategoryListQueryParams,
    req: Parts,
) -> AppResult<Json<CategoryListResponse>> {
    // FIXME: There are 69 categories, 47 top level. This isn't going to
    // grow by an OoM. We need a limit for /summary, but we don't need
    // to paginate this.
    let options = PaginationOptions::builder().gather(&req)?;

    let conn = app.db_read().await?;

    let sort = params.sort.as_ref().map_or("alpha", String::as_str);

    let offset = options.offset().unwrap_or_default();

    let (categories, total) = tokio::try_join!(
        Category::toplevel(&conn, sort, options.per_page, offset),
        Category::count_toplevel(&conn),
    )?;

    let categories = categories.into_iter().map(Category::into).collect();

    let meta = CategoryListMeta { total };
    Ok(Json(CategoryListResponse { categories, meta }))
}

/// Response returned when getting category metadata.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CategoryGetResponse {
    pub category: EncodableCategory,
}

/// Get category metadata.
#[utoipa::path(
    get,
    path = "/api/v1/categories/{category}",
    params(
        ("category" = String, Path, description = "Name of the category"),
    ),
    tag = "categories",
    responses(
        (status = 200, description = "Successful Response", body = inline(CategoryGetResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn find_category(
    state: AppState,
    Path(slug): Path<String>,
) -> AppResult<Json<CategoryGetResponse>> {
    // Category slugs can never contain null bytes, so we reject such requests
    // early with a regular "not found" response instead of letting them reach
    // the database layer, where PostgreSQL rejects the query with a confusing
    // `invalid byte sequence for encoding "UTF8": 0x00` error.
    if slug.contains('\0') {
        return Err(not_found());
    }

    let mut conn = state.db_read().await?;

    let cat: Category = Category::by_slug(&slug)
        .select(Category::as_select())
        .first(&mut conn)
        .await?;
    let (subcats, parents) =
        tokio::try_join!(cat.subcategories(&conn), cat.parent_categories(&conn),)?;

    let subcats = subcats.into_iter().map(Category::into).collect();
    let parents = parents.into_iter().map(Category::into).collect();

    let mut category = EncodableCategory::from(cat);
    category.subcategories = Some(subcats);
    category.parent_categories = Some(parents);

    Ok(Json(CategoryGetResponse { category }))
}

#[derive(Debug, Serialize, Queryable, utoipa::ToSchema)]
pub struct Slug {
    /// An opaque identifier for the category.
    #[schema(example = "game-development")]
    id: String,

    /// The "slug" of the category.
    ///
    /// See <https://crates.io/category_slugs>.
    #[schema(example = "game-development")]
    slug: String,

    /// A description of the category.
    #[schema(example = "Libraries for creating games.")]
    description: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ListSlugsResponse {
    /// The list of category slugs.
    pub category_slugs: Vec<Slug>,
}

/// List all available category slugs.
#[utoipa::path(
    get,
    path = "/api/v1/category_slugs",
    tag = "categories",
    responses(
        (status = 200, description = "Successful Response", body = inline(ListSlugsResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn list_category_slugs(state: AppState) -> AppResult<Json<ListSlugsResponse>> {
    let mut conn = state.db_read().await?;

    let category_slugs = categories::table
        .select((categories::slug, categories::slug, categories::description))
        .order(categories::slug)
        .load(&mut conn)
        .await?;

    Ok(Json(ListSlugsResponse { category_slugs }))
}
