use crate::util::errors::AppResult;
use axum::response::{IntoResponse, Response};
use axum::Json;

pub(crate) mod pagination;

pub(crate) use self::pagination::Paginate;

pub fn ok_true() -> AppResult<Response> {
    let json = json!({ "ok": true });
    Ok(Json(json).into_response())
}
