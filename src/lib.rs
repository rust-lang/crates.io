extern crate conduit;
extern crate hyper;

use std::net::SocketAddr;

use hyper::rt::Future;
use hyper::service::service_fn_ok;
use hyper::{Body, Request, Response, Server};

#[derive(Debug)]
struct Headers<'a>(&'a hyper::HeaderMap);

impl<'a> conduit::Headers for Headers<'a> {
    /// Find all values associated with a header, or None.
    ///
    /// If the value of a header is not valid UTF-8, that value
    /// is replaced with the emtpy string.
    fn find(&self, key: &str) -> Option<Vec<&str>> {
        let values = self.0
            .get_all(key)
            .iter()
            .map(|v| v.to_str().unwrap_or(""))
            .collect::<Vec<&str>>();

        if values.is_empty() {
            None
        } else {
            Some(values)
        }
    }

    fn has(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    /// Returns a representation of all headers
    ///
    /// There is currently a bug where keys with mutliple values will be duplicated.
    /// See: https://github.com/hyperium/http/issues/199
    fn all(&self) -> Vec<(&str, Vec<&str>)> {
        let keys = self.0.keys();
        let mut all = Vec::new();
        for key in keys {
            let key = key.as_str();
            let values = self.find(key)
                .expect("all keys should have at least one value");
            all.push((key, values));
        }
        all
    }
}

pub fn run(addr: SocketAddr) {
    let new_svc = || service_fn_ok(handler);

    let server = Server::bind(&addr).serve(new_svc);
    hyper::rt::run(server.map_err(|_| ()));
}

fn handler(request: Request<Body>) -> Response<Body> {
    use conduit::Headers;
    let headers = Headers(request.headers());
    println!("{:?}", request.headers().keys_len());
    Response::new(Body::from(format!(
        "all: {:?}\nfind A: {:?}\n",
        headers.all(),
        headers.find("A")
    )))
}
