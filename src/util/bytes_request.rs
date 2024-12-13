use crate::middleware::log_request::ErrorField;
use axum::body::Bytes;
use axum::extract::{FromRequest, Request};
use axum::response::{IntoResponse, Response};
use axum::{async_trait, Extension, RequestExt};
use derive_more::{Deref, DerefMut};
use http::StatusCode;
use http_body_util::{BodyExt, LengthLimitError};
use std::error::Error;

#[derive(Debug, Deref, DerefMut)]
pub struct BytesRequest(pub Request<Bytes>);

#[async_trait]
impl<S> FromRequest<S> for BytesRequest
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let req = req.with_limited_body();
        let (parts, body) = req.into_parts();

        let collected = body.collect().await.map_err(|err| {
            let box_error = err.into_inner();
            match box_error.downcast::<LengthLimitError>() {
                Ok(_) => StatusCode::PAYLOAD_TOO_LARGE.into_response(),
                Err(err) => server_error_response(&*err),
            }
        })?;
        let bytes = collected.to_bytes();

        let request = Request::from_parts(parts, bytes);

        Ok(BytesRequest(request))
    }
}

/// Logs an error message and returns a generic status 500 response
fn server_error_response<E: Error + ?Sized>(error: &E) -> Response {
    error!(%error, "Internal Server Error");

    sentry::capture_error(error);

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Extension(ErrorField(error.to_string())),
        "Internal Server Error",
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::BytesRequest;
    use axum::extract::DefaultBodyLimit;
    use axum::routing::get;
    use axum::Router;
    use http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn content_length_too_large() {
        const BODY_SIZE_LIMIT: usize = 4096;

        fn app() -> Router {
            async fn bytes_request(_req: BytesRequest) {}

            Router::new()
                .route("/", get(bytes_request))
                .layer(DefaultBodyLimit::max(BODY_SIZE_LIMIT))
        }

        let body = vec![0; BODY_SIZE_LIMIT + 1];
        let body = axum::body::Body::from(body);
        let request = Request::get("/").body(body).unwrap();
        let response = app().oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);

        let body = vec![0; BODY_SIZE_LIMIT];
        let body = axum::body::Body::from(body);
        let request = Request::get("/").body(body).unwrap();
        let response = app().oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
