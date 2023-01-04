use crate::file_stream::FileStream;
use axum::response::IntoResponse;
use http::Response;

pub type AxumResponse = axum::response::Response;
pub type ConduitResponse = Response<conduit::Body>;

/// Turns a `ConduitResponse` into a `AxumResponse`
pub fn conduit_into_axum(response: ConduitResponse) -> AxumResponse {
    use conduit::Body::*;

    let (parts, body) = response.into_parts();
    match body {
        Static(slice) => Response::from_parts(parts, axum::body::Body::from(slice)).into_response(),
        Owned(vec) => Response::from_parts(parts, axum::body::Body::from(vec)).into_response(),
        File(file) => Response::from_parts(parts, FileStream::from_std(file).into_streamed_body())
            .into_response(),
    }
}
