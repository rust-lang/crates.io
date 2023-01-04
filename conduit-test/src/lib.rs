use bytes::Bytes;
use hyper::http::{header::IntoHeaderName, HeaderValue, Method, Request};
use std::io::Cursor;

pub struct MockRequest {
    request: Request<Cursor<Bytes>>,
}

impl MockRequest {
    pub fn new(method: Method, path: &str) -> MockRequest {
        let request = Request::builder()
            .method(&method)
            .uri(path)
            .body(Cursor::new(Bytes::new()))
            .unwrap();

        MockRequest { request }
    }

    pub fn with_body(&mut self, bytes: &[u8]) -> &mut MockRequest {
        *self.request.body_mut() = Cursor::new(bytes.to_vec().into());
        self
    }

    pub fn header<K>(&mut self, name: K, value: &str) -> &mut MockRequest
    where
        K: IntoHeaderName,
    {
        self.request
            .headers_mut()
            .insert(name, HeaderValue::from_str(value).unwrap());
        self
    }

    pub fn into_inner(self) -> Request<Cursor<Bytes>> {
        self.request
    }
}

impl From<MockRequest> for Request<hyper::Body> {
    fn from(mock_request: MockRequest) -> Self {
        let (parts, body) = mock_request.request.into_parts();
        Request::from_parts(parts, hyper::Body::from(body.into_inner()))
    }
}

#[cfg(test)]
mod tests {
    use super::MockRequest;

    use hyper::http::{header, Method};

    #[test]
    fn simple_request_test() {
        let req = MockRequest::new(Method::GET, "/").into_inner();

        assert_eq!(req.method(), Method::GET);
        assert_eq!(req.uri(), "/");
        assert_eq!(req.headers().len(), 0);
        assert_eq!(req.body().get_ref(), "");
    }

    #[test]
    fn request_body_test() {
        let mut req = MockRequest::new(Method::POST, "/articles");
        req.with_body(b"Hello world");
        let req = req.into_inner();

        assert_eq!(req.method(), Method::POST);
        assert_eq!(req.uri(), "/articles");
        assert_eq!(req.body().get_ref(), "Hello world");
    }

    #[test]
    fn request_query_test() {
        let req = MockRequest::new(Method::POST, "/articles?foo=bar").into_inner();
        assert_eq!(req.uri().query().expect("No query string"), "foo=bar");
    }

    #[test]
    fn request_headers() {
        let mut req = MockRequest::new(Method::POST, "/articles");
        req.header(header::USER_AGENT, "lulz");
        req.header(header::DNT, "1");
        let req = req.into_inner();

        assert_eq!(req.headers().len(), 2);
        assert_eq!(req.headers().get(header::USER_AGENT).unwrap(), "lulz");
        assert_eq!(req.headers().get(header::DNT).unwrap(), "1");
    }
}
