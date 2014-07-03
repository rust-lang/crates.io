use std::io::MemReader;
use std::collections::HashMap;

use conduit::{Request, Response};

pub trait RequestRedirect {
    fn redirect(self, url: String) -> Response;
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
