use axum::body::Bytes;
use http::{HeaderValue, Request, header, header::IntoHeaderName};

pub type MockRequest = Request<Bytes>;

pub trait MockRequestExt {
    fn header<K: IntoHeaderName>(&mut self, name: K, value: &str);
    fn with_body(self, bytes: Bytes) -> Self;
}

impl MockRequestExt for MockRequest {
    fn header<K>(&mut self, name: K, value: &str)
    where
        K: IntoHeaderName,
    {
        self.headers_mut()
            .append(name, HeaderValue::from_str(value).unwrap());
    }

    fn with_body(mut self, bytes: Bytes) -> Self {
        if is_json_body(&bytes) {
            self.header(header::CONTENT_TYPE, "application/json");
        }

        *self.body_mut() = bytes;
        self
    }
}

fn is_json_body(body: &Bytes) -> bool {
    (body.starts_with(b"{") && body.ends_with(b"}"))
        || (body.starts_with(b"[") && body.ends_with(b"]"))
}

#[cfg(test)]
mod tests {
    use super::{MockRequest, MockRequestExt};

    use axum::body::Bytes;
    use hyper::http::{Method, Request, header};

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
        *req.body_mut() = Bytes::from_static(b"Hello world");

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
