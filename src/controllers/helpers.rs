use crate::controllers::cargo_prelude::{AppResult, Response};
use crate::util::json_response;
use axum::response::IntoResponse;

pub(crate) mod pagination;

pub(crate) use self::pagination::Paginate;

pub fn ok_true() -> AppResult<Response> {
    let json = json!({ "ok": true });
    Ok(json_response(json).into_response())
}
