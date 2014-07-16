use std::io::{MemReader, IoError};
use std::collections::HashMap;
use std::fmt::Show;

use serialize::{json, Encodable};
use url;

use conduit::{Request, Response, Handler};

pub use self::errors::{CargoError, CargoResult, internal, internal_error};
pub use self::errors::{ChainError, BoxError};
pub use self::result::{Require, Wrap};

pub mod errors;
pub mod result;

pub trait RequestUtils {
    fn redirect(self, url: String) -> Response;

    fn json<'a, T: Encodable<json::Encoder<'a>, IoError>>(self, t: &T) -> Response;
    fn query(self) -> HashMap<String, String>;
}

impl<'a> RequestUtils for &'a Request {
    fn json<'a, T: Encodable<json::Encoder<'a>, IoError>>(self, t: &T) -> Response {
        let s = json::encode(t);
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(),
                       vec!["application/json; charset=utf-8".to_string()]);
        Response {
            status: (200, "OK"),
            headers: headers,
            body: box MemReader::new(s.into_bytes()),
        }
    }

    fn query(self) -> HashMap<String, String> {
        self.query_string().unwrap_or("").split('&').filter_map(|s| {
            let mut parts = s.split('=');
            let k = parts.next().unwrap_or(s);
            let v = parts.next().unwrap_or("");
            let k = try_option!(url::decode_component(k).ok());
            let v = try_option!(url::decode_component(v).ok());
            Some((k, v))
        }).collect()
    }

    fn redirect(self, url: String) -> Response {
        let mut headers = HashMap::new();
        headers.insert("Location".to_string(), vec![url.to_string()]);
        Response {
            status: (302, "Found"),
            headers: headers,
            body: box MemReader::new(Vec::new()),
        }
    }
}

pub struct C(pub fn(&mut Request) -> CargoResult<Response>);

impl Handler for C {
    fn call(&self, req: &mut Request) -> Result<Response, Box<Show>> {
        let C(f) = *self;
        match f(req) {
            Ok(req) => Ok(req),
            Err(e) => match e.response() {
                Some(response) => Ok(response),
                None => Err(box e as Box<Show>),
            }
        }
    }
}
