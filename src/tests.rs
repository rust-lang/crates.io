use std::collections::HashMap;
use std::error::Error;
use std::io::Cursor;

use conduit::{Handler, Request, Response};
use futures::{Future, Stream};
use hyper;

struct OkResult;
impl Handler for OkResult {
    fn call(&self, _req: &mut Request) -> Result<Response, Box<Error + Send>> {
        Ok(Response {
            status: (200, "OK"),
            headers: build_headers("value"),
            body: Box::new(Cursor::new("Hello, world!")),
        })
    }
}

struct ErrorResult;
impl Handler for ErrorResult {
    fn call(&self, _req: &mut Request) -> Result<Response, Box<Error + Send>> {
        let error = ::std::io::Error::last_os_error();
        Err(Box::new(error))
    }
}

struct Panic;
impl Handler for Panic {
    fn call(&self, _req: &mut Request) -> Result<Response, Box<Error + Send>> {
        panic!()
    }
}

struct InvalidHeader;
impl Handler for InvalidHeader {
    fn call(&self, _req: &mut Request) -> Result<Response, Box<Error + Send>> {
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
    fn call(&self, _req: &mut Request) -> Result<Response, Box<Error + Send>> {
        Ok(Response {
            status: (1000, "invalid status code"),
            headers: build_headers("discarded"),
            body: Box::new(Cursor::new("discarded")),
        })
    }
}

fn build_headers(msg: &str) -> HashMap<String, Vec<String>> {
    let mut headers = HashMap::new();
    headers.insert("ok".into(), vec![msg.into()]);
    headers
}

fn simulate_request<H: Handler>(handler: H) -> hyper::Response<hyper::Body> {
    use hyper::service::{NewService, Service};

    let new_service = super::Service::new(handler, 1);
    let mut service = new_service.new_service().wait().unwrap();
    service.call(hyper::Request::default()).wait().unwrap()
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
