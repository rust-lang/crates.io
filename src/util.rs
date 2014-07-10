use std::io::{MemReader, IoError};
use std::collections::HashMap;
use serialize::{json, Encodable};
use url;

use conduit::{Request, Response};

pub trait RequestUtils {
    fn not_found(self) -> Response;
    fn unauthorized(self) -> Response;
    fn redirect(self, url: String) -> Response;

    fn json<'a, T: Encodable<json::Encoder<'a>, IoError>>(self, t: &T) -> Response;
    fn query(self) -> HashMap<String, String>;
}

impl<'a> RequestUtils for &'a mut Request {
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

    fn not_found(self) -> Response {
        Response {
            status: (404, "Not Found"),
            headers: HashMap::new(),
            body: box MemReader::new(Vec::new()),
        }
    }

    fn unauthorized(self) -> Response {
        Response {
            status: (403, "Forbidden"),
            headers: HashMap::new(),
            body: box MemReader::new(Vec::new()),
        }
    }
}
