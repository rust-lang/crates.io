use crate::controllers::cargo_prelude::{AppResult, Response};
use axum::response::IntoResponse;
use axum::Json;

pub(crate) mod pagination;

pub(crate) use self::pagination::Paginate;

pub fn ok_true() -> AppResult<Response> {
    let json = json!({ "ok": true });
    Ok(Json(json).into_response())
}
