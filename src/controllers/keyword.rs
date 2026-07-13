use crate::app::AppState;
use crate::controllers::helpers::pagination::{PaginationOptions, PaginationQueryParams};
use crate::controllers::helpers::{Paginate, pagination::Paginated};
use crate::models::Keyword;
use crate::util::errors::{AppResult, not_found};
use crate::views::EncodableKeyword;
use axum::Json;
use axum::extract::{FromRequestParts, Path, Query};
use diesel::prelude::*;
use http::request::Parts;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, FromRequestParts, utoipa::IntoParams)]
#[from_request(via(Query))]
#[into_params(parameter_in = Query)]
pub struct ListQueryParams {
    /// The sort order of the keywords.
    ///
    /// Valid values: `alpha`, and `crates`.
    ///
    /// Defaults to `alpha`.
    sort: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ListResponse {
    /// The list of keywords.
    pub keywords: Vec<EncodableKeyword>,

    #[schema(inline)]
    pub meta: ListMeta,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ListMeta {
    /// The total number of keywords.
    #[schema(example = 123)]
    pub total: i64,
}

/// List all keywords.
#[utoipa::path(
    get,
    path = "/api/v1/keywords",
    params(ListQueryParams, PaginationQueryParams),
    tag = "keywords",
    responses((status = 200, description = "Successful Response", body = inline(ListResponse))),
)]
pub async fn list_keywords(
    state: AppState,
    params: ListQueryParams,
    req: Parts,
) -> AppResult<Json<ListResponse>> {
    use crate::schema::keywords;

    let mut query = Keyword::query().into_boxed();

    query = match &params.sort {
        Some(sort) if sort == "crates" => query.order(keywords::crates_cnt.desc()),
        _ => query.order(keywords::keyword.asc()),
    };

    let query = query.pages_pagination(PaginationOptions::builder().gather(&req)?);

    let mut conn = state.db_read().await?;
    let data: Paginated<Keyword> = query.load(&mut conn).await?;
    let total = data.total();
    let keywords = data.into_iter().map(Keyword::into).collect();

    let meta = ListMeta { total };
    Ok(Json(ListResponse { keywords, meta }))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct GetResponse {
    pub keyword: EncodableKeyword,
}

/// Get keyword metadata.
#[utoipa::path(
    get,
    path = "/api/v1/keywords/{keyword}",
    params(
        ("keyword" = String, Path, description = "The keyword to find"),
    ),
    tag = "keywords",
    responses((status = 200, description = "Successful Response", body = inline(GetResponse))),
)]
pub async fn find_keyword(
    Path(name): Path<String>,
    state: AppState,
) -> AppResult<Json<GetResponse>> {
    // If the name is not a valid keyword it cannot exist in the database, so we
    // skip the lookup and return a regular "not found" response. This also
    // avoids passing invalid input (e.g. names containing null bytes) to the
    // database layer, where PostgreSQL would reject the query with a confusing
    // `invalid byte sequence for encoding "UTF8": 0x00` error and cause a 500
    // response.
    if !Keyword::valid_name(&name) {
        return Err(not_found());
    }

    let conn = state.db_read().await?;
    let kw = Keyword::find_by_keyword(&conn, &name).await?;
    let keyword = EncodableKeyword::from(kw);
    Ok(Json(GetResponse { keyword }))
}
