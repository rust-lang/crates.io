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

/// Query parameters for listing keywords.
#[derive(Debug, Deserialize, FromRequestParts, utoipa::IntoParams)]
#[from_request(via(Query))]
#[into_params(parameter_in = Query)]
pub struct KeywordListQueryParams {
    /// The sort order of the keywords.
    ///
    /// Valid values: `alpha`, and `crates`.
    ///
    /// Defaults to `alpha`.
    sort: Option<String>,
}

/// Response returned when listing keywords.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct KeywordListResponse {
    /// The list of keywords.
    pub keywords: Vec<EncodableKeyword>,

    #[schema(inline)]
    pub meta: KeywordListMeta,
}

/// Pagination metadata for a keyword list response.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct KeywordListMeta {
    /// The total number of keywords.
    #[schema(example = 123)]
    pub total: i64,
}

/// List all keywords.
#[utoipa::path(
    get,
    path = "/api/v1/keywords",
    params(KeywordListQueryParams, PaginationQueryParams),
    tag = "keywords",
    responses((status = 200, description = "Successful Response", body = inline(KeywordListResponse))),
)]
pub async fn list_keywords(
    state: AppState,
    params: KeywordListQueryParams,
    req: Parts,
) -> AppResult<Json<KeywordListResponse>> {
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

    let meta = KeywordListMeta { total };
    Ok(Json(KeywordListResponse { keywords, meta }))
}

/// Response returned when getting keyword metadata.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct KeywordGetResponse {
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
    responses((status = 200, description = "Successful Response", body = inline(KeywordGetResponse))),
)]
pub async fn find_keyword(
    Path(name): Path<String>,
    state: AppState,
) -> AppResult<Json<KeywordGetResponse>> {
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
    Ok(Json(KeywordGetResponse { keyword }))
}
