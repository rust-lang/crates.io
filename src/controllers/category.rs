use super::helpers::pagination::*;
use crate::app::AppState;
use crate::models::Category;
use crate::schema::categories;
use crate::util::errors::AppResult;
use crate::views::EncodableCategory;
use axum::Json;
use axum::extract::{FromRequestParts, Path, Query};
use diesel::QueryDsl;
use diesel_async::RunQueryDsl;
use futures_util::FutureExt;
use http::request::Parts;

#[derive(Debug, Deserialize, FromRequestParts, utoipa::IntoParams)]
#[from_request(via(Query))]
#[into_params(parameter_in = Query)]
pub struct ListQueryParams {
    /// The sort order of the categories.
    ///
    /// Valid values: `alpha`, and `crates`.
    ///
    /// Defaults to `alpha`.
    sort: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ListResponse {
    /// The list of categories.
    pub categories: Vec<EncodableCategory>,

    #[schema(inline)]
    pub meta: ListMeta,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ListMeta {
    /// The total number of categories.
    #[schema(example = 123)]
    pub total: i64,
}

/// List all categories.
#[utoipa::path(
    get,
    path = "/api/v1/categories",
    params(ListQueryParams, PaginationQueryParams),
    tag = "categories",
    responses((status = 200, description = "Successful Response", body = inline(ListResponse))),
)]
pub async fn list_categories(
    app: AppState,
    params: ListQueryParams,
    req: Parts,
) -> AppResult<Json<ListResponse>> {
    // FIXME: There are 69 categories, 47 top level. This isn't going to
    // grow by an OoM. We need a limit for /summary, but we don't need
    // to paginate this.
    let options = PaginationOptions::builder().gather(&req)?;

    let mut conn = app.db_read().await?;

    let sort = params.sort.as_ref().map_or("alpha", String::as_str);

    let offset = options.offset().unwrap_or_default();

    let (categories, total) = tokio::try_join!(
        Category::toplevel(&mut conn, sort, options.per_page, offset).boxed(),
        // Query for the total count of categories
        Category::count_toplevel(&mut conn).boxed(),
    )?;

    let categories = categories.into_iter().map(Category::into).collect();

    let meta = ListMeta { total };
    Ok(Json(ListResponse { categories, meta }))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct GetResponse {
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
    responses((status = 200, description = "Successful Response", body = inline(GetResponse))),
)]
pub async fn find_category(
    state: AppState,
    Path(slug): Path<String>,
) -> AppResult<Json<GetResponse>> {
    let mut conn = state.db_read().await?;

    let cat: Category = Category::by_slug(&slug).first(&mut conn).await?;
    let (subcats, parents) = tokio::try_join!(
        cat.subcategories(&mut conn),
        cat.parent_categories(&mut conn).boxed(),
    )?;

    let subcats = subcats.into_iter().map(Category::into).collect();
    let parents = parents.into_iter().map(Category::into).collect();

    let mut category = EncodableCategory::from(cat);
    category.subcategories = Some(subcats);
    category.parent_categories = Some(parents);

    Ok(Json(GetResponse { category }))
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
    responses((status = 200, description = "Successful Response", body = inline(ListSlugsResponse))),
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
