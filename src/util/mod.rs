use std::collections::HashMap;
use std::error::Error;
use std::io::Cursor;
use std::sync::Arc;

use serde_json;
use serde::Serialize;

use conduit::{Handler, Request, Response};
use conduit_router::{RequestParams, RouteBuilder};
use self::errors::NotFound;

pub use self::errors::{bad_request, human, internal, internal_error, CargoError, CargoResult};
pub use self::errors::{std_error, ChainError};
pub use self::hasher::{hash, HashingReader};
pub use self::io_util::{read_fill, LimitErrorReader, read_le_u32};
pub use self::request_proxy::RequestProxy;

pub mod errors;
pub mod rfc3339;
mod hasher;
mod io_util;
mod request_proxy;

pub fn json_response<T: Serialize>(t: &T) -> Response {
    let json = serde_json::to_string(t).unwrap();
    let mut headers = HashMap::new();
    headers.insert(
        "Content-Type".to_string(),
        vec!["application/json; charset=utf-8".to_string()],
    );
    headers.insert("Content-Length".to_string(), vec![json.len().to_string()]);
    Response {
        status: (200, "OK"),
        headers: headers,
        body: Box::new(Cursor::new(json.into_bytes())),
    }
}

// Can't Copy or Debug the fn.
#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct C(pub fn(&mut Request) -> CargoResult<Response>);

impl Handler for C {
    fn call(&self, req: &mut Request) -> Result<Response, Box<Error + Send>> {
        let C(f) = *self;
        match f(req) {
            Ok(resp) => Ok(resp),
            Err(e) => match e.response() {
                Some(response) => Ok(response),
                None => Err(std_error(e)),
            },
        }
    }
}

#[derive(Debug)]
pub struct R<H>(pub Arc<H>);

impl<H: Handler> Handler for R<H> {
    fn call(&self, req: &mut Request) -> Result<Response, Box<Error + Send>> {
        let path = req.params()["path"].to_string();
        let R(ref sub_router) = *self;
        sub_router.call(&mut RequestProxy {
            other: req,
            path: Some(&path),
            method: None,
        })
    }
}

// Can't derive Debug because of RouteBuilder.
#[allow(missing_debug_implementations)]
pub struct R404(pub RouteBuilder);

impl Handler for R404 {
    fn call(&self, req: &mut Request) -> Result<Response, Box<Error + Send>> {
        let R404(ref router) = *self;
        match router.recognize(&req.method(), req.path()) {
            Ok(m) => {
                req.mut_extensions().insert(m.params.clone());
                m.handler.call(req)
            }
            Err(..) => Ok(NotFound.response().unwrap()),
        }
    }
}
