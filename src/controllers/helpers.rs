use crate::util::errors::AppResult;
use axum::response::{IntoResponse, Response};
use axum_extra::json;

pub(crate) mod pagination;

pub(crate) use self::pagination::Paginate;

pub fn ok_true() -> AppResult<Response> {
    let json = json!({ "ok": true });
    Ok(json.into_response())
}
