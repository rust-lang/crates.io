use axum::Json;
use axum::response::{IntoResponse, Response};

pub mod authorization;
pub(crate) mod pagination;

pub(crate) use self::pagination::Paginate;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct OkResponse {
    #[schema(example = true)]
    ok: bool,
}

impl Default for OkResponse {
    fn default() -> Self {
        Self::new()
    }
}

impl OkResponse {
    pub fn new() -> Self {
        Self { ok: true }
    }
}

impl IntoResponse for OkResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
