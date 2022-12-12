use std::net::SocketAddr;

use axum::extract::ConnectInfo;
use axum::{Extension, Router};
use conduit::{box_error, Body, Handler, HandlerResult, RequestExt};
use http::{HeaderValue, Request, Response, StatusCode};
use hyper::{body::to_bytes, service::Service};
use tokio::{sync::oneshot, task::JoinHandle};

use crate::{AxumResponse, ConduitFallback};

struct OkResult;
impl Handler for OkResult {
    fn call(&self, _req: &mut dyn RequestExt) -> HandlerResult {
        Response::builder()
            .header("ok", "value")
            .body(Body::from_static(b"Hello, world!"))
            .map_err(box_error)
    }
}

struct ErrorResult;
impl Handler for ErrorResult {
    fn call(&self, _req: &mut dyn RequestExt) -> HandlerResult {
        let error = ::std::io::Error::last_os_error();
        Err(Box::new(error))
    }
}

struct Panic;
impl Handler for Panic {
    fn call(&self, _req: &mut dyn RequestExt) -> HandlerResult {
        panic!()
    }
}

struct InvalidHeader;
impl Handler for InvalidHeader {
    fn call(&self, _req: &mut dyn RequestExt) -> HandlerResult {
        Response::builder()
            .header("invalid-value", "\r\n")
            .body(Body::from_static(b"discarded"))
            .map_err(box_error)
    }
}

struct InvalidStatus;
impl Handler for InvalidStatus {
    fn call(&self, _req: &mut dyn RequestExt) -> HandlerResult {
        Response::builder()
            .status(1000)
            .body(Body::empty())
            .map_err(box_error)
    }
}

struct Sleep;
impl Handler for Sleep {
    fn call(&self, req: &mut dyn RequestExt) -> HandlerResult {
        std::thread::sleep(std::time::Duration::from_millis(100));
        OkResult.call(req)
    }
}

struct AssertPercentDecodedPath;
impl Handler for AssertPercentDecodedPath {
    fn call(&self, req: &mut dyn RequestExt) -> HandlerResult {
        if req.path() == "/:" && req.query_string() == Some("%3a") {
            OkResult.call(req)
        } else {
            ErrorResult.call(req)
        }
    }
}

fn make_service<H: Handler>(handler: H) -> Router {
    let remote_addr: SocketAddr = ([0, 0, 0, 0], 0).into();

    Router::new()
        .conduit_fallback(handler)
        .layer(Extension(ConnectInfo(remote_addr)))
}

async fn simulate_request<H: Handler>(handler: H) -> AxumResponse {
    let mut service = make_service(handler);
    service.call(Request::default()).await.unwrap()
}

async fn assert_generic_err(resp: AxumResponse) {
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(resp.headers().len(), 1);
    assert_eq!(
        resp.headers().get("content-length"),
        Some(&HeaderValue::from_static("21"))
    );
    let full_body = to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(&*full_body, b"Internal Server Error");
}

#[tokio::test]
async fn valid_ok_response() {
    let resp = simulate_request(OkResult).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers().len(), 2);
    assert!(resp.headers().get("ok").is_some());
    assert!(resp.headers().get("content-length").is_some());
    let full_body = to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(&*full_body, b"Hello, world!");
}

#[tokio::test]
async fn invalid_ok_responses() {
    assert_generic_err(simulate_request(InvalidHeader).await).await;
    assert_generic_err(simulate_request(InvalidStatus).await).await;
}

#[tokio::test]
async fn err_responses() {
    assert_generic_err(simulate_request(ErrorResult).await).await;
}

#[ignore] // catch_unwind not yet implemented
#[tokio::test]
async fn recover_from_panic() {
    assert_generic_err(simulate_request(Panic).await).await;
}

#[tokio::test]
async fn sleeping_doesnt_block_another_request() {
    let mut service = make_service(Sleep);

    let first = service.call(hyper::Request::default());
    let second = service.call(hyper::Request::default());

    let start = std::time::Instant::now();

    // Spawn 2 requests that each sleeps for 100ms
    let (first, second) = futures_util::join!(first, second);

    // Elapsed time should be closer to 100ms than 200ms
    assert!(start.elapsed().as_millis() < 150);

    assert_eq!(first.unwrap().status(), StatusCode::OK);
    assert_eq!(second.unwrap().status(), StatusCode::OK);
}

#[tokio::test]
async fn path_is_percent_decoded_but_not_query_string() {
    let mut service = make_service(AssertPercentDecodedPath);
    let req = hyper::Request::put("/%3a?%3a")
        .body(hyper::Body::default())
        .unwrap();
    let resp = service.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

async fn spawn_http_server() -> (
    String,
    JoinHandle<Result<(), hyper::Error>>,
    oneshot::Sender<()>,
) {
    let (quit_tx, quit_rx) = oneshot::channel::<()>();
    let addr = ([127, 0, 0, 1], 0).into();

    let router = Router::new().conduit_fallback(OkResult);
    let make_service = router.into_make_service_with_connect_info::<SocketAddr>();
    let server = hyper::Server::bind(&addr).serve(make_service);

    let url = format!("http://{}", server.local_addr());
    let server = server.with_graceful_shutdown(async {
        quit_rx.await.ok();
    });

    (url, tokio::spawn(server), quit_tx)
}

#[tokio::test]
async fn content_length_too_large() {
    const ACTUAL_BODY_SIZE: usize = 10_000;
    const CLAIMED_CONTENT_LENGTH: u64 = 11_111_111_111_111_111_111;

    let (url, server, quit_tx) = spawn_http_server().await;

    let client = hyper::Client::new();
    let (mut sender, body) = hyper::Body::channel();
    sender
        .send_data(vec![0; ACTUAL_BODY_SIZE].into())
        .await
        .unwrap();
    let req = hyper::Request::put(url)
        .header(hyper::header::CONTENT_LENGTH, CLAIMED_CONTENT_LENGTH)
        .body(body)
        .unwrap();

    let resp = client
        .request(req)
        .await
        .expect("should be a valid response");

    quit_tx.send(()).unwrap();
    server.await.unwrap().unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
