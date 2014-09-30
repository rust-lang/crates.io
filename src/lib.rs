#![feature(globs)]

extern crate semver;
extern crate conduit;

use std::io::net::ip::{IpAddr, Ipv4Addr};
use std::io::MemReader;
use std::collections::HashMap;
use std::fmt::Show;
use std::path::BytesContainer;

use semver::Version;
use conduit::{Method, Scheme, Host, Extensions, Headers, TypeMap};

pub struct MockRequest {
    path: String,
    method: Method,
    query_string: Option<String>,
    body: Option<Vec<u8>>,
    build_headers: HashMap<String, String>,
    headers: MockHeaders,
    extensions: TypeMap,
    reader: Option<MemReader>
}

impl MockRequest {
    pub fn new(method: Method, path: &str) -> MockRequest {
        let headers = HashMap::new();

        MockRequest {
            path: path.to_string(),
            extensions: TypeMap::new(),
            query_string: None,
            body: None,
            build_headers: headers,
            headers: MockHeaders { headers: HashMap::new() },
            method: method,
            reader: None
        }
    }

    pub fn with_path(&mut self, path: &str) -> &mut MockRequest {
        self.path = path.to_string();
        self
    }

    pub fn with_query<'a, S: Show>(&'a mut self, string: S) -> &'a mut MockRequest {
        self.query_string = Some(string.to_string());
        self
    }

    pub fn with_body<'a, S: BytesContainer>(&'a mut self, string: S) -> &'a mut MockRequest {
        self.body = Some(string.container_as_bytes().to_vec());
        self
    }

    pub fn header<'a, S1: Show, S2: Show>(&'a mut self, name: S1, value: S2) -> &'a mut MockRequest {
        self.build_headers.insert(name.to_string(), value.to_string());
        let headers = MockHeaders { headers: self.build_headers.clone() };
        self.headers = headers;

        self
    }
}

pub struct MockHeaders {
    headers: HashMap<String, String>
}

impl Headers for MockHeaders {
    fn find<'a>(&'a self, key: &str) -> Option<Vec<&'a str>> {
        self.headers.find_equiv(&key).map(|v| vec!(v.as_slice()))
    }

    fn has(&self, key: &str) -> bool {
        self.headers.contains_key_equiv(&key)
    }

    fn iter<'a>(&'a self) -> conduit::HeaderEntries<'a> {
        box self.headers.iter().map(|(k,v)| (k.as_slice(), vec!(v.as_slice()))) as conduit::HeaderEntries<'a>
    }
}

impl<'a> conduit::Request for MockRequest {
    fn http_version(&self) -> Version {
        Version::parse("1.1.0").unwrap()
    }

    fn conduit_version(&self) -> Version {
        Version::parse("0.1.0").unwrap()
    }

    fn method(&self) -> Method { self.method }
    fn scheme(&self) -> Scheme { conduit::Http }
    fn host<'a>(&'a self) -> Host<'a> { conduit::HostName("example.com") }
    fn virtual_root<'a>(&'a self) -> Option<&'a str> { None }

    fn path<'a>(&'a self) -> &'a str {
        self.path.as_slice()
    }

    fn query_string<'a>(&'a self) -> Option<&'a str> {
        self.query_string.as_ref().map(|s| s.as_slice())
    }

    fn remote_ip(&self) -> IpAddr {
        Ipv4Addr(127, 0, 0, 1)
    }

    fn content_length(&self) -> Option<uint> {
        self.body.as_ref().map(|b| b.len())
    }

    fn headers<'a>(&'a self) -> &'a Headers {
        &self.headers as &Headers
    }

    fn body<'a>(&'a mut self) -> &'a mut Reader {
        let body = self.body.clone().unwrap_or(Vec::new());
        self.reader = Some(MemReader::new(body));

        self.reader.as_mut().unwrap() as &mut Reader
    }

    fn extensions<'a>(&'a self) -> &'a Extensions {
        &self.extensions
    }
    fn mut_extensions<'a>(&'a mut self) -> &'a mut Extensions {
        &mut self.extensions
    }
}

#[cfg(test)]
mod tests {
    use super::MockRequest;
    use conduit;
    use semver::Version;

    use std::io::net::ip::Ipv4Addr;

    use conduit::Request;

    #[test]
    fn simple_request_test() {
        let mut req = MockRequest::new(conduit::Get, "/");

        assert_eq!(req.http_version(), Version::parse("1.1.0").unwrap());
        assert_eq!(req.conduit_version(), Version::parse("0.1.0").unwrap());
        assert_eq!(req.method(), conduit::Get);
        assert_eq!(req.scheme(), conduit::Http);
        assert_eq!(req.host(), conduit::HostName("example.com"));
        assert_eq!(req.virtual_root(), None);
        assert_eq!(req.path(), "/");
        assert_eq!(req.query_string(), None);
        assert_eq!(req.remote_ip(), Ipv4Addr(127, 0, 0, 1));
        assert_eq!(req.content_length(), None);
        assert_eq!(req.headers().iter().count(), 0);
        assert_eq!(req.body().read_to_string().ok().expect("No body"),
                   "".to_string());
    }

    #[test]
    fn request_body_test() {
        let mut req = MockRequest::new(conduit::Post, "/articles");
        req.with_body("Hello world");

        assert_eq!(req.method(), conduit::Post);
        assert_eq!(req.path(), "/articles");
        assert_eq!(req.body().read_to_string().ok().expect("No body"),
                   "Hello world".to_string());
        assert_eq!(req.content_length(), Some(11));
    }

    #[test]
    fn request_query_test() {
        let mut req = MockRequest::new(conduit::Post, "/articles");
        req.with_query("foo=bar");

        assert_eq!(req.query_string().expect("No query string"), "foo=bar");
    }

    #[test]
    fn request_headers() {
        let mut req = MockRequest::new(conduit::Post, "/articles");
        req.header("User-Agent", "lulz");
        req.header("DNT", "1");

        assert_eq!(req.headers().iter().count(), 2);
        assert_eq!(req.headers().find("User-Agent").unwrap(), vec!("lulz"));
        assert_eq!(req.headers().find("DNT").unwrap(), vec!("1"));
    }
}
