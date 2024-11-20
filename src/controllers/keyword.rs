use crate::app::AppState;
use crate::controllers::helpers::pagination::PaginationOptions;
use crate::controllers::helpers::{pagination::Paginated, Paginate};
use crate::models::Keyword;
use crate::util::errors::AppResult;
use crate::views::EncodableKeyword;
use axum::extract::{Path, Query};
use axum_extra::json;
use axum_extra::response::ErasedJson;
use diesel::prelude::*;
use http::request::Parts;

#[derive(Deserialize)]
pub struct IndexQuery {
    sort: Option<String>,
}

/// Handles the `GET /keywords` route.
pub async fn index(state: AppState, qp: Query<IndexQuery>, req: Parts) -> AppResult<ErasedJson> {
    use crate::schema::keywords;

    let mut query = keywords::table.into_boxed();

    query = match &qp.sort {
        Some(sort) if sort == "crates" => query.order(keywords::crates_cnt.desc()),
        _ => query.order(keywords::keyword.asc()),
    };

    let query = query.pages_pagination(PaginationOptions::builder().gather(&req)?);

    let mut conn = state.db_read().await?;
    let data: Paginated<Keyword> = query.async_load(&mut conn).await?;
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

/// Handles the `GET /keywords/:keyword_id` route.
pub async fn show(Path(name): Path<String>, state: AppState) -> AppResult<ErasedJson> {
    let mut conn = state.db_read().await?;
    let kw = Keyword::find_by_keyword(&mut conn, &name).await?;

    Ok(json!({ "keyword": EncodableKeyword::from(kw) }))
}
