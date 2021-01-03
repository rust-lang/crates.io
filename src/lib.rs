#![warn(rust_2018_idioms)]
extern crate conduit;

use std::borrow::Cow;
use std::io::{Cursor, Read};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use conduit::{
    header::{HeaderValue, IntoHeaderName},
    Body, Extensions, HeaderMap, Host, Method, Response, Scheme, Version,
};

pub trait ResponseExt {
    fn into_cow(self) -> Cow<'static, [u8]>;
}

impl ResponseExt for Response<Body> {
    /// Convert the request into a copy-on-write body
    ///
    /// # Blocking
    ///
    /// This function may block if the value is a `Body::File`.
    ///
    /// # Panics
    ///
    /// This function panics if there is an error reading a `Body::File`.
    fn into_cow(self) -> Cow<'static, [u8]> {
        use conduit::Body::*;

        match self.into_body() {
            Static(slice) => slice.into(),
            Owned(vec) => vec.into(),
            File(mut file) => {
                let mut vec = Vec::new();
                std::io::copy(&mut file, &mut vec).unwrap();
                vec.into()
            }
        }
    }
}

pub struct MockRequest {
    path: String,
    method: Method,
    query_string: Option<String>,
    body: Option<Vec<u8>>,
    headers: HeaderMap,
    extensions: Extensions,
    reader: Option<Cursor<Vec<u8>>>,
}

impl MockRequest {
    pub fn new(method: Method, path: &str) -> MockRequest {
        let headers = HeaderMap::new();

        MockRequest {
            path: path.to_string(),
            extensions: Extensions::new(),
            query_string: None,
            body: None,
            headers,
            method,
            reader: None,
        }
    }

    pub fn with_method(&mut self, method: Method) -> &mut MockRequest {
        self.method = method;
        self
    }

    pub fn with_path(&mut self, path: &str) -> &mut MockRequest {
        self.path = path.to_string();
        self
    }

    pub fn with_query(&mut self, string: &str) -> &mut MockRequest {
        self.query_string = Some(string.to_string());
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
    fn scheme(&self) -> Scheme {
        Scheme::Http
    }
    fn host(&self) -> Host<'_> {
        Host::Name("example.com")
    }
    fn virtual_root(&self) -> Option<&str> {
        None
    }

    fn path(&self) -> &str {
        &self.path
    }

    fn path_mut(&mut self) -> &mut String {
        &mut self.path
    }

    fn query_string(&self) -> Option<&str> {
        self.query_string.as_ref().map(|s| &s[..])
    }

    fn remote_addr(&self) -> SocketAddr {
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 80))
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

    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

    use conduit::{header, Host, Method, RequestExt, Scheme, Version};

    #[test]
    fn simple_request_test() {
        let mut req = MockRequest::new(Method::GET, "/");

        assert_eq!(req.http_version(), Version::HTTP_11);
        assert_eq!(req.method(), Method::GET);
        assert_eq!(req.scheme(), Scheme::Http);
        assert_eq!(req.host(), Host::Name("example.com"));
        assert_eq!(req.virtual_root(), None);
        assert_eq!(req.path(), "/");
        assert_eq!(req.query_string(), None);
        assert_eq!(
            req.remote_addr(),
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 80))
        );
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
        assert_eq!(req.path(), "/articles");
        let mut s = String::new();
        req.body().read_to_string(&mut s).expect("No body");
        assert_eq!(s, "Hello world".to_string());
        assert_eq!(req.content_length(), Some(11));
    }

    #[test]
    fn request_query_test() {
        let mut req = MockRequest::new(Method::POST, "/articles");
        req.with_query("foo=bar");

        assert_eq!(req.query_string().expect("No query string"), "foo=bar");
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
