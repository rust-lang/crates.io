extern crate hyper;

use std::collections::HashMap;
use std::net::SocketAddr;

use hyper::rt::Future;
use hyper::service::service_fn_ok;
use hyper::{Body, Request, Response, Server};

pub fn run(addr: SocketAddr) {
    let new_svc = || service_fn_ok(handler);

    let server = Server::bind(&addr).serve(new_svc);
    hyper::rt::run(server.map_err(|_| ()));
}

fn handler(request: Request<Body>) -> Response<Body> {
    let mut headers = HashMap::new();
    for (key, value) in request.headers() {
        headers
            .entry(key.as_str().to_string())
            .or_insert_with(Vec::new)
            .push(value.to_str().unwrap_or("").to_string());
    }

    Response::new(Body::from(format!("{:?}", headers)))
}
