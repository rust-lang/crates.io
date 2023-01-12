use crate::{server_error_response, spawn_blocking, BytesRequest, ServiceError};
use axum::extract::DefaultBodyLimit;
use axum::response::{IntoResponse, Response};
use axum::Router;
use http::header::HeaderName;
use http::{HeaderMap, HeaderValue, Request, StatusCode, Uri};
use hyper::body::to_bytes;
use tokio::{sync::oneshot, task::JoinHandle};

fn single_header(key: &str, value: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(key.parse::<HeaderName>().unwrap(), value.parse().unwrap());
    headers
}

async fn ok_result() -> Response {
    (single_header("ok", "value"), "Hello, world!").into_response()
}

async fn error_result() -> Response {
    server_error_response(&std::io::Error::last_os_error())
}

async fn panic() -> Response {
    panic!()
}

async fn sleep() -> Result<Response, ServiceError> {
    spawn_blocking(move || std::thread::sleep(std::time::Duration::from_millis(100)))
        .await
        .map_err(ServiceError::from)?;

    Ok(ok_result().await)
}

async fn assert_percent_decode_path(uri: Uri) -> Response {
    if uri.path() == "/%3a" && uri.query() == Some("%3a") {
        ok_result().await
    } else {
        error_result().await
    }
}

async fn conduit_request(_req: BytesRequest) {}

async fn assert_generic_err(resp: Response) {
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(resp.headers().len(), 1);
    assert_eq!(
        resp.headers().get("content-type"),
        Some(&HeaderValue::from_static("text/plain; charset=utf-8"))
    );
    let full_body = to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(&*full_body, b"Internal Server Error");
}

#[tokio::test]
async fn valid_ok_response() {
    let resp = ok_result().await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers().len(), 2);
    assert!(resp.headers().get("ok").is_some());
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );
    let full_body = to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(&*full_body, b"Hello, world!");
}

#[tokio::test]
async fn err_responses() {
    assert_generic_err(error_result().await).await;
}

#[ignore] // catch_unwind not yet implemented
#[tokio::test]
async fn recover_from_panic() {
    assert_generic_err(panic().await).await;
}

#[tokio::test]
async fn sleeping_doesnt_block_another_request() {
    let first = sleep();
    let second = sleep();

    let start = std::time::Instant::now();

    // Spawn 2 requests that each sleeps for 100ms
    let (first, second) = futures_util::join!(first, second);

    // Elapsed time should be closer to 100ms than 200ms
    dbg!(start.elapsed().as_millis());
    assert!(start.elapsed().as_millis() < 150);

    assert_eq!(first.unwrap().status(), StatusCode::OK);
    assert_eq!(second.unwrap().status(), StatusCode::OK);
}

#[tokio::test]
async fn path_is_percent_decoded_but_not_query_string() {
    let req = Request::put("/%3a?%3a").body(()).unwrap();
    let resp = assert_percent_decode_path(req.uri().clone()).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

async fn spawn_http_server() -> (
    String,
    JoinHandle<Result<(), hyper::Error>>,
    oneshot::Sender<()>,
) {
    let (quit_tx, quit_rx) = oneshot::channel::<()>();
    let addr = ([127, 0, 0, 1], 0).into();

    let router = Router::new()
        .fallback(conduit_request)
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
