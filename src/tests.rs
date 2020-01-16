use std::collections::HashMap;
use std::error::Error;
use std::io::Cursor;

use conduit::{Handler, Request, Response};
use futures::prelude::*;
use hyper::{body::to_bytes, service::Service};

use super::service::{BlockingHandler, ServiceError};

struct OkResult;
impl Handler for OkResult {
    fn call(&self, _req: &mut dyn Request) -> Result<Response, Box<dyn Error + Send>> {
        Ok(Response {
            status: (200, "OK"),
            headers: build_headers("value"),
            body: Box::new(Cursor::new("Hello, world!")),
        })
    }
}

struct ErrorResult;
impl Handler for ErrorResult {
    fn call(&self, _req: &mut dyn Request) -> Result<Response, Box<dyn Error + Send>> {
        let error = ::std::io::Error::last_os_error();
        Err(Box::new(error))
    }
}

struct Panic;
impl Handler for Panic {
    fn call(&self, _req: &mut dyn Request) -> Result<Response, Box<dyn Error + Send>> {
        panic!()
    }
}

struct InvalidHeader;
impl Handler for InvalidHeader {
    fn call(&self, _req: &mut dyn Request) -> Result<Response, Box<dyn Error + Send>> {
        let mut headers = build_headers("discarded");
        headers.insert("invalid".into(), vec!["\r\n".into()]);
        Ok(Response {
            status: (200, "OK"),
            headers,
            body: Box::new(Cursor::new("discarded")),
        })
    }
}

struct InvalidStatus;
impl Handler for InvalidStatus {
    fn call(&self, _req: &mut dyn Request) -> Result<Response, Box<dyn Error + Send>> {
        Ok(Response {
            status: (1000, "invalid status code"),
            headers: build_headers("discarded"),
            body: Box::new(Cursor::new("discarded")),
        })
    }
}

struct AssertPathNormalized;
impl Handler for AssertPathNormalized {
    fn call(&self, req: &mut dyn Request) -> Result<Response, Box<dyn Error + Send>> {
        if req.path() == "/normalized" {
            OkResult.call(req)
        } else {
            ErrorResult.call(req)
        }
    }
}

struct Sleep;
impl Handler for Sleep {
    fn call(&self, req: &mut dyn Request) -> Result<Response, Box<dyn Error + Send>> {
        std::thread::sleep(std::time::Duration::from_millis(100));
        OkResult.call(req)
    }
}

fn build_headers(msg: &str) -> HashMap<String, Vec<String>> {
    let mut headers = HashMap::new();
    headers.insert("ok".into(), vec![msg.into()]);
    headers
}

fn make_service<H: Handler>(
    handler: H,
) -> impl Service<
    hyper::Request<hyper::Body>,
    Response = hyper::Response<hyper::Body>,
    Future = impl Future<Output = Result<hyper::Response<hyper::Body>, ServiceError>> + Send + 'static,
    Error = ServiceError,
> {
    use hyper::service::service_fn;

    let handler = std::sync::Arc::new(BlockingHandler::new(handler, 1));

    service_fn(move |request: hyper::Request<hyper::Body>| {
        let remote_addr = ([0, 0, 0, 0], 0).into();
        handler.clone().blocking_handler(request, remote_addr)
    })
}

async fn simulate_request<H: Handler>(handler: H) -> hyper::Response<hyper::Body> {
    let mut service = make_service(handler);
    service.call(hyper::Request::default()).await.unwrap()
}

async fn assert_generic_err(resp: hyper::Response<hyper::Body>) {
    assert_eq!(resp.status(), 500);
    assert!(resp.headers().is_empty());
    let full_body = to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(&*full_body, b"Internal Server Error");
}

#[tokio::test]
async fn valid_ok_response() {
    let resp = simulate_request(OkResult).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers().len(), 1);
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
async fn normalize_path() {
    let mut service = make_service(AssertPathNormalized);
    let req = hyper::Request::put("//removed/.././.././normalized")
        .body(hyper::Body::default())
        .unwrap();
    let resp = service.call(req).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers().len(), 1);

    let req = hyper::Request::put("//normalized")
        .body(hyper::Body::default())
        .unwrap();
    let resp = service.call(req).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers().len(), 1);
}

#[tokio::test]
async fn limits_thread_count() {
    let mut service = make_service(Sleep);
    let first = service.call(hyper::Request::default());
    let second = service.call(hyper::Request::default());

    let first_completed = futures::select! {
        // The first thead is spawned and sleeps for 100ms
        sleep = first.fuse() => sleep,
        // The second request is rejected immediately
        over_capacity = second.fuse() => over_capacity,
    }.unwrap();

    assert_eq!(first_completed.status(), 503)
}
