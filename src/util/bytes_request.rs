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
    use axum::Router;
    use http::StatusCode;
    use tokio::sync::oneshot;
    use tokio::task::JoinHandle;

    async fn bytes_request(_req: BytesRequest) {}

    async fn spawn_http_server() -> (
        String,
        JoinHandle<Result<(), hyper::Error>>,
        oneshot::Sender<()>,
    ) {
        let (quit_tx, quit_rx) = oneshot::channel::<()>();
        let addr = ([127, 0, 0, 1], 0).into();

        let router = Router::new()
            .fallback(bytes_request)
            .layer(DefaultBodyLimit::max(4096));
        let make_service = router.into_make_service();
        let server = hyper::Server::bind(&addr).serve(make_service);

        let url = format!("http://{}", server.local_addr());
        let server = server.with_graceful_shutdown(async {
            quit_rx.await.ok();
        });

        (url, tokio::spawn(server), quit_tx)
    }

    #[tokio::test]
    async fn content_length_too_large() {
        const ACTUAL_BODY_SIZE: usize = 4097;

        let (url, server, quit_tx) = spawn_http_server().await;

        let client = hyper::Client::new();
        let (mut sender, body) = hyper::Body::channel();
        sender
            .send_data(vec![0; ACTUAL_BODY_SIZE].into())
            .await
            .unwrap();
        let req = hyper::Request::put(url).body(body).unwrap();

        let resp = client
            .request(req)
            .await
            .expect("should be a valid response");

        quit_tx.send(()).unwrap();
        server.await.unwrap().unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
