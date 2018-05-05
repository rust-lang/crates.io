extern crate hyper;

use std::net::SocketAddr;

use hyper::rt::Future;
use hyper::service::service_fn_ok;
use hyper::{Body, Response, Server};

static TEXT: &str = "Hello, World!";

pub struct ConduitServer;

impl ConduitServer {
    pub fn run(addr: SocketAddr) {
        let new_svc = || service_fn_ok(|_req| Response::new(Body::from(TEXT)));

        let server = Server::bind(&addr).serve(new_svc);
        hyper::rt::run(server.map_err(|_| ()));
    }
}
