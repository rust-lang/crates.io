use crate::middleware::log_request::ErrorField;
use axum::body::Bytes;
use axum::extract::FromRequest;
use axum::response::{IntoResponse, Response};
use axum::{async_trait, Extension, RequestExt};
use http::{Request, StatusCode};
use http_body::{Body, LengthLimitError};
use std::error::Error;
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct BytesRequest(pub Request<Bytes>);

impl Deref for BytesRequest {
    type Target = Request<Bytes>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BytesRequest {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[async_trait]
impl<S, B> FromRequest<S, B> for BytesRequest
where
    S: Send + Sync,
    B: Body + Send + 'static,
    B::Data: Send,
    B::Error: Into<Box<dyn Error + Send + Sync>>,
{
    type Rejection = Response;

    async fn from_request(req: Request<B>, _state: &S) -> Result<Self, Self::Rejection> {
        let request = match req.with_limited_body() {
            Ok(req) => {
                let (parts, body) = req.into_parts();

                let bytes = hyper::body::to_bytes(body).await.map_err(|err| {
                    if err.downcast_ref::<LengthLimitError>().is_some() {
                        StatusCode::BAD_REQUEST.into_response()
                    } else {
                        server_error_response(&*err)
                    }
                })?;

                Request::from_parts(parts, bytes)
            }
            Err(req) => {
                let (parts, body) = req.into_parts();

                let bytes = hyper::body::to_bytes(body)
                    .await
                    .map_err(|err| server_error_response(&*err.into()))?;

                Request::from_parts(parts, bytes)
            }
        };

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

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = vec![0; BODY_SIZE_LIMIT];
        let body = axum::body::Body::from(body);
        let request = Request::get("/").body(body).unwrap();
        let response = app().oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
