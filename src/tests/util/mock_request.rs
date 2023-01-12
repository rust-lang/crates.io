use axum::body::Bytes;
use http::{header::IntoHeaderName, HeaderValue, Request};

pub type MockRequest = Request<Bytes>;

pub trait MockRequestExt {
    fn with_body(&mut self, bytes: &[u8]);

    fn header<K: IntoHeaderName>(&mut self, name: K, value: &str);
}

impl MockRequestExt for MockRequest {
    fn with_body(&mut self, bytes: &[u8]) {
        *self.body_mut() = bytes.to_vec().into();
    }

    fn header<K>(&mut self, name: K, value: &str)
    where
        K: IntoHeaderName,
    {
        self.headers_mut()
            .insert(name, HeaderValue::from_str(value).unwrap());
    }
}

#[cfg(test)]
mod tests {
    use super::{MockRequest, MockRequestExt};

    use axum::body::Bytes;
    use hyper::http::{header, Method, Request};

    pub fn mock_request(method: Method, path: &str) -> MockRequest {
        Request::builder()
            .method(&method)
            .uri(path)
            .body(Bytes::new())
            .unwrap()
    }

    #[test]
    fn simple_request_test() {
        let req = mock_request(Method::GET, "/");

        assert_eq!(req.method(), Method::GET);
        assert_eq!(req.uri(), "/");
        assert_eq!(req.headers().len(), 0);
        assert_eq!(req.body(), "");
    }

    #[test]
    fn request_body_test() {
        let mut req = mock_request(Method::POST, "/articles");
        req.with_body(b"Hello world");

        assert_eq!(req.method(), Method::POST);
        assert_eq!(req.uri(), "/articles");
        assert_eq!(req.body(), "Hello world");
    }

    #[test]
    fn request_query_test() {
        let req = mock_request(Method::POST, "/articles?foo=bar");
        assert_eq!(req.uri().query().expect("No query string"), "foo=bar");
    }

    #[test]
    fn request_headers() {
        let mut req = mock_request(Method::POST, "/articles");
        req.header(header::USER_AGENT, "lulz");
        req.header(header::DNT, "1");

        assert_eq!(req.headers().len(), 2);
        assert_eq!(req.headers().get(header::USER_AGENT).unwrap(), "lulz");
        assert_eq!(req.headers().get(header::DNT).unwrap(), "1");
    }
}
