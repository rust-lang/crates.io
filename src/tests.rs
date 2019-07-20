use std::collections::HashMap;
use std::error::Error;
use std::io::Cursor;

use conduit::{Handler, Request, Response};
use futures::{Future, Stream};
use hyper;

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

fn build_headers(msg: &str) -> HashMap<String, Vec<String>> {
    let mut headers = HashMap::new();
    headers.insert("ok".into(), vec![msg.into()]);
    headers
}

fn build_threadpool() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new()
        .core_threads(1)
        .blocking_threads(1)
        .build()
        .unwrap()
}

fn simulate_request<H: Handler>(handler: H) -> hyper::Response<hyper::Body> {
    use hyper::service::{NewService, Service};

    let mut pool = build_threadpool();

    pool.block_on(futures::lazy(|| {
        let new_service = super::Service::new(handler);
        let mut service = new_service.new_service().wait().unwrap();
        service.call(hyper::Request::default()).wait()
    }))
    .unwrap()
}

fn into_chunk(resp: hyper::Response<hyper::Body>) -> hyper::Chunk {
    resp.into_body().concat2().wait().unwrap()
}

fn assert_generic_err(resp: hyper::Response<hyper::Body>) {
    assert_eq!(resp.status(), 500);
    assert!(resp.headers().is_empty());
    let full_body = into_chunk(resp);
    assert_eq!(&*full_body, b"Internal Server Error");
}

#[test]
fn valid_ok_response() {
    let resp = simulate_request(OkResult);
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers().len(), 1);
    let full_body = into_chunk(resp);
    assert_eq!(&*full_body, b"Hello, world!");
}

#[test]
fn invalid_ok_responses() {
    assert_generic_err(simulate_request(InvalidHeader));
    assert_generic_err(simulate_request(InvalidStatus));
}

#[test]
fn err_responses() {
    assert_generic_err(simulate_request(ErrorResult));
}

#[ignore] // catch_unwind not yet implemented
#[test]
fn recover_from_panic() {
    assert_generic_err(simulate_request(Panic));
}

#[test]
fn normalize_path() {
    use hyper::service::{NewService, Service};

    let mut pool = build_threadpool();

    pool.block_on(futures::lazy(|| {
        let new_service = super::Service::new(AssertPathNormalized);
        let mut service = new_service.new_service().wait().unwrap();
        let req = hyper::Request::put("//removed/.././.././normalized")
            .body(hyper::Body::default())
            .unwrap();
        let resp = service.call(req).wait().unwrap();
        assert_eq!(resp.status(), 200);
        assert_eq!(resp.headers().len(), 1);

        let req = hyper::Request::put("//normalized")
            .body(hyper::Body::default())
            .unwrap();
        let resp = service.call(req).wait().unwrap();
        assert_eq!(resp.status(), 200);
        assert_eq!(resp.headers().len(), 1);
        Ok::<_, ()>(())
    }))
    .unwrap()
}
