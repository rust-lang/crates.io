use super::prelude::*;
use crate::app::AppState;
use axum::extract::{Path, State};
use axum::Json;

use crate::controllers::helpers::pagination::PaginationOptions;
use crate::controllers::helpers::{pagination::Paginated, Paginate};
use crate::models::Keyword;
use crate::views::EncodableKeyword;

/// Handles the `GET /keywords` route.
pub async fn index(req: ConduitRequest) -> AppResult<Json<Value>> {
    conduit_compat(move || {
        use crate::schema::keywords;

        let query = req.query();
        let sort = query.get("sort").map(|s| &s[..]).unwrap_or("alpha");

        let mut query = keywords::table.into_boxed();

        if sort == "crates" {
            query = query.order(keywords::crates_cnt.desc());
        } else {
            query = query.order(keywords::keyword.asc());
        }

        let query = query.pages_pagination(PaginationOptions::builder().gather(&req)?);
        let conn = req.app().db_read()?;
        let data: Paginated<Keyword> = query.load(&conn)?;
        let total = data.total();
        let kws = data
            .into_iter()
            .map(Keyword::into)
            .collect::<Vec<EncodableKeyword>>();

        Ok(Json(json!({
            "keywords": kws,
            "meta": { "total": total },
        })))
    })
    .await
}

/// Handles the `GET /keywords/:keyword_id` route.
pub async fn show(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> AppResult<Json<Value>> {
    conduit_compat(move || {
        let conn = state.db_read()?;

        let kw = Keyword::find_by_keyword(&conn, &name)?;

        Ok(Json(json!({ "keyword": EncodableKeyword::from(kw) })))
    })
    .await
}
