use std::borrow::Cow;
use std::io::{Cursor, Read};

use conduit::{
    header::{HeaderValue, IntoHeaderName},
    Body, Extensions, HeaderMap, Method, Response, Uri, Version,
};

pub trait ResponseExt {
    fn into_cow(self) -> Cow<'static, [u8]>;
}

impl ResponseExt for Response<Body> {
    /// Convert the request into a copy-on-write body
    fn into_cow(self) -> Cow<'static, [u8]> {
        use conduit::Body::*;

        match self.into_body() {
            Static(slice) => slice.into(),
            Owned(vec) => vec.into(),
        }
    }
}

fn uri(path_and_query: &str) -> Uri {
    Uri::builder()
        .path_and_query(path_and_query)
        .build()
        .unwrap()
}

pub struct MockRequest {
    method: Method,
    uri: Uri,
    body: Option<Vec<u8>>,
    headers: HeaderMap,
    extensions: Extensions,
    reader: Option<Cursor<Vec<u8>>>,
}

impl MockRequest {
    pub fn new(method: Method, path: &str) -> MockRequest {
        let headers = HeaderMap::new();
        let extensions = Extensions::new();

        MockRequest {
            uri: uri(path),
            extensions,
            body: None,
            headers,
            method,
            reader: None,
        }
    }

    pub fn with_query(&mut self, string: &str) -> &mut MockRequest {
        let path_and_query = format!("{}?{}", self.uri.path(), string);
        self.uri = uri(&path_and_query);
        self
    }

    pub fn with_body(&mut self, bytes: &[u8]) -> &mut MockRequest {
        self.body = Some(bytes.to_vec());
        self.reader = None;
        self
    }

    pub fn header<K>(&mut self, name: K, value: &str) -> &mut MockRequest
    where
        K: IntoHeaderName,
    {
        self.headers
            .insert(name, HeaderValue::from_str(value).unwrap());
        self
    }
}

impl conduit::RequestExt for MockRequest {
    fn http_version(&self) -> Version {
        Version::HTTP_11
    }

    fn method(&self) -> &Method {
        &self.method
    }

    fn uri(&self) -> &Uri {
        &self.uri
    }

    fn content_length(&self) -> Option<u64> {
        self.body.as_ref().map(|b| b.len() as u64)
    }

    fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    fn body(&mut self) -> &mut dyn Read {
        if self.reader.is_none() {
            let body = self.body.clone().unwrap_or_default();
            self.reader = Some(Cursor::new(body));
        }
        self.reader.as_mut().unwrap()
    }

    fn extensions(&self) -> &Extensions {
        &self.extensions
    }
    fn mut_extensions(&mut self) -> &mut Extensions {
        &mut self.extensions
    }
}

#[cfg(test)]
mod tests {
    use super::MockRequest;

    use conduit::{header, Method, RequestExt, Version};

    #[test]
    fn simple_request_test() {
        let mut req = MockRequest::new(Method::GET, "/");

        assert_eq!(req.http_version(), Version::HTTP_11);
        assert_eq!(req.method(), Method::GET);
        assert_eq!(req.uri(), "/");
        assert_eq!(req.content_length(), None);
        assert_eq!(req.headers().len(), 0);
        let mut s = String::new();
        req.body().read_to_string(&mut s).expect("No body");
        assert_eq!(s, "".to_string());
    }

    #[test]
    fn request_body_test() {
        let mut req = MockRequest::new(Method::POST, "/articles");
        req.with_body(b"Hello world");

        assert_eq!(req.method(), Method::POST);
        assert_eq!(req.uri(), "/articles");
        let mut s = String::new();
        req.body().read_to_string(&mut s).expect("No body");
        assert_eq!(s, "Hello world".to_string());
        assert_eq!(req.content_length(), Some(11));
    }

    #[test]
    fn request_query_test() {
        let mut req = MockRequest::new(Method::POST, "/articles");
        req.with_query("foo=bar");

        assert_eq!(req.uri().query().expect("No query string"), "foo=bar");
    }

    #[test]
    fn request_headers() {
        let mut req = MockRequest::new(Method::POST, "/articles");
        req.header(header::USER_AGENT, "lulz");
        req.header(header::DNT, "1");

        assert_eq!(req.headers().len(), 2);
        assert_eq!(req.headers().get(header::USER_AGENT).unwrap(), "lulz");
        assert_eq!(req.headers().get(header::DNT).unwrap(), "1");
    }
}
