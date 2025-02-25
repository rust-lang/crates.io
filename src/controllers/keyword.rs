use crate::app::AppState;
use crate::controllers::helpers::pagination::{PaginationOptions, PaginationQueryParams};
use crate::controllers::helpers::{Paginate, pagination::Paginated};
use crate::models::Keyword;
use crate::util::errors::AppResult;
use crate::views::EncodableKeyword;
use axum::extract::{FromRequestParts, Path, Query};
use axum_extra::json;
use axum_extra::response::ErasedJson;
use diesel::prelude::*;
use http::request::Parts;

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

/// List all keywords.
#[utoipa::path(
    get,
    path = "/api/v1/keywords",
    params(ListQueryParams, PaginationQueryParams),
    tag = "keywords",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn list_keywords(
    state: AppState,
    params: ListQueryParams,
    req: Parts,
) -> AppResult<ErasedJson> {
    use crate::schema::keywords;

    let mut query = keywords::table.into_boxed();

    query = match &params.sort {
        Some(sort) if sort == "crates" => query.order(keywords::crates_cnt.desc()),
        _ => query.order(keywords::keyword.asc()),
    };

    let query = query.pages_pagination(PaginationOptions::builder().gather(&req)?);

    let mut conn = state.db_read().await?;
    let data: Paginated<Keyword> = query.load(&mut conn).await?;
    let total = data.total();
    let kws = data
        .into_iter()
        .map(Keyword::into)
        .collect::<Vec<EncodableKeyword>>();

    Ok(json!({
        "keywords": kws,
        "meta": { "total": total },
    }))
}

/// Get keyword metadata.
#[utoipa::path(
    get,
    path = "/api/v1/keywords/{keyword}",
    params(
        ("keyword" = String, Path, description = "The keyword to find"),
    ),
    tag = "keywords",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn find_keyword(Path(name): Path<String>, state: AppState) -> AppResult<ErasedJson> {
    let mut conn = state.db_read().await?;
    let kw = Keyword::find_by_keyword(&mut conn, &name).await?;

    Ok(json!({ "keyword": EncodableKeyword::from(kw) }))
}
