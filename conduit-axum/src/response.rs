use crate::body;
use axum::response::IntoResponse;
use http::Response;

pub type AxumResponse = axum::response::Response;
pub type ConduitResponse = Response<conduit::Body>;

/// Turns a `ConduitResponse` into a `AxumResponse`
pub fn conduit_into_axum(response: ConduitResponse) -> AxumResponse {
    let (parts, body) = response.into_parts();
    Response::from_parts(parts, body::conduit_into_axum(body)).into_response()
}
