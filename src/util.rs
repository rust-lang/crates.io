use std::io::{MemReader, IoError};
use std::collections::HashMap;
use serialize::{json, Encodable};

use conduit::{Request, Response};

pub trait RequestRedirect {
    fn redirect(self, url: String) -> Response;
}

pub trait RequestJson {
    fn json<'a, T: Encodable<json::Encoder<'a>, IoError>>(self, t: &T) -> Response;
}

impl<'a> RequestRedirect for &'a mut Request {
    fn redirect(self, url: String) -> Response {
        let mut headers = HashMap::new();
        headers.insert("Location".to_string(), vec![url.to_str()]);
        Response {
            status: (302, "Found"),
            headers: headers,
            body: box MemReader::new(Vec::new()),
        }
    }
}

impl<'a> RequestJson for &'a mut Request {
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
}
