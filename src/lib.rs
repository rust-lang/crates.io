#![cfg_attr(test, deny(warnings))]

extern crate conduit;
extern crate conduit_middleware as middleware;
extern crate time;

use conduit::{Method, Request};
use middleware::Middleware;
use std::borrow::Cow;
use std::error::Error;
use std::io;
use time::{ParseError, Tm};

pub type Response = Result<conduit::Response, Box<dyn Error + Send>>;

#[allow(missing_copy_implementations)]
pub struct ConditionalGet;

impl Middleware for ConditionalGet {
    fn after(&self, req: &mut dyn Request, res: Response) -> Response {
        let mut res = res?;

        match req.method() {
            Method::Get | Method::Head => {
                if is_ok(&res) && is_fresh(req, &res) {
                    res.status = (304, "Not Modified");
                    res.headers.remove("Content-Type");
                    res.headers.remove("Content-Length");
                    res.body = Box::new(io::empty());
                }
            }
            _ => (),
        }

        Ok(res)
    }
}

fn is_ok(response: &conduit::Response) -> bool {
    match *response {
        conduit::Response {
            status: (200, _), ..
        } => true,
        _ => false,
    }
}

fn is_fresh(req: &dyn Request, res: &conduit::Response) -> bool {
    let modified_since = req.headers().find("If-Modified-Since").map(header_val);
    let none_match = req.headers().find("If-None-Match").map(header_val);

    if modified_since
        .as_ref()
        .or_else(|| none_match.as_ref())
        .is_none()
    {
        return false;
    }

    let mut success = true;

    modified_since
        .and_then(|modified_since| {
            parse_http_date(&modified_since)
                .map_err(|_| {
                    success = false;
                })
                .ok()
        })
        .map(|parsed| {
            success = success && is_modified_since(parsed, res);
        });

    none_match.map(|none_match| {
        success = success && etag_matches(&none_match, res);
    });

    success
}

fn etag_matches(none_match: &str, res: &conduit::Response) -> bool {
    res.headers
        .get("ETag")
        .map(|etag| res_header_val(etag) == none_match)
        .unwrap_or(false)
}

fn is_modified_since(modified_since: Tm, res: &conduit::Response) -> bool {
    res.headers
        .get("Last-Modified")
        .and_then(|last_modified| parse_http_date(&res_header_val(last_modified)).ok())
        .map(|last_modified| modified_since.to_timespec() >= last_modified.to_timespec())
        .unwrap_or(false)
}

fn header_val<'a>(header: Vec<&'a str>) -> Cow<'a, str> {
    if header.len() == 1 {
        Cow::Borrowed(header[0])
    } else {
        Cow::Owned(header.concat())
    }
}

fn res_header_val<'a>(header: &'a Vec<String>) -> Cow<'a, str> {
    if header.len() == 1 {
        Cow::Borrowed(&header[0])
    } else {
        Cow::Owned(header.concat())
    }
}

fn parse_http_date(string: &str) -> Result<Tm, ()> {
    parse_rfc1123(string)
        .or_else(|_| parse_rfc850(string))
        .or_else(|_| parse_asctime(string))
        .map_err(|_| ())
}

fn parse_rfc1123(string: &str) -> Result<Tm, ParseError> {
    time::strptime(string, "%a, %d %b %Y %T GMT")
}

fn parse_rfc850(string: &str) -> Result<Tm, ParseError> {
    time::strptime(string, "%a, %d-%m-%y %T GMT")
}

fn parse_asctime(string: &str) -> Result<Tm, ParseError> {
    time::strptime(string, "%a %m%t%d %T %Y")
}

#[cfg(test)]
mod tests {
    extern crate conduit_test as test;

    use conduit::{Handler, Method, Request, Response};
    use middleware::MiddlewareBuilder;
    use std::collections::HashMap;
    use std::error::Error;
    use std::io::Cursor;
    use time;
    use time::Tm;

    use super::ConditionalGet;

    macro_rules! returning {
        ($code:expr, $($header:expr => $value:expr),+) => ({
            let mut headers = HashMap::new();
            $(headers.insert($header.to_string(), vec!($value.to_string()));)+
            let handler = SimpleHandler::new(headers, $code, "hello");
            let mut stack = MiddlewareBuilder::new(handler);
            stack.add(ConditionalGet);
            stack
        });
        ($($header:expr => $value:expr),+) => ({
            returning!((200, "OK"), $($header => $value),+)
        })
    }

    macro_rules! request {
        ($($header:expr => $value:expr),+) => ({
            let mut req = test::MockRequest::new(Method::Get, "/");
            $(req.header($header, &$value.to_string());)+
            req
        })
    }

    #[test]
    fn test_sends_304() {
        let handler = returning!("Last-Modified" => httpdate(time::now()));
        expect_304(handler.call(&mut request!(
            "If-Modified-Since" => httpdate(time::now())
        )));
    }

    #[test]
    fn test_sends_304_if_older_than_now() {
        let handler = returning!("Last-Modified" => before_now());
        expect_304(handler.call(&mut request!(
            "If-Modified-Since" => httpdate(time::now())
        )));
    }

    #[test]
    fn test_sends_304_with_etag() {
        let handler = returning!("ETag" => "1234");
        expect_304(handler.call(&mut request!(
            "If-None-Match" => "1234"
        )));
    }

    #[test]
    fn test_sends_200_with_fresh_time_but_not_etag() {
        let handler = returning!("Last-Modified" => before_now(), "ETag" => "1234");
        expect_200(handler.call(&mut request!(
            "If-Modified-Since" => now(),
            "If-None-Match" => "4321"
        )));
    }

    #[test]
    fn test_sends_200_with_fresh_etag_but_not_time() {
        let handler = returning!("Last-Modified" => now(), "ETag" => "1234");
        expect_200(handler.call(&mut request!(
            "If-Modified-Since" => before_now(),
            "If-None-Match" => "1234"
        )));
    }

    #[test]
    fn test_sends_200_with_fresh_etag() {
        let handler = returning!("ETag" => "1234");
        expect_200(handler.call(&mut request!(
            "If-None-Match" => "4321"
        )));
    }

    #[test]
    fn test_sends_200_with_fresh_time() {
        let handler = returning!("Last-Modified" => now());
        expect_200(handler.call(&mut request!(
            "If-Modified-Since" => before_now()
        )));
    }

    #[test]
    fn test_sends_304_with_fresh_time_and_etag() {
        let handler = returning!("Last-Modified" => before_now(), "ETag" => "1234");
        expect_304(handler.call(&mut request!(
            "If-Modified-Since" => now(),
            "If-None-Match" => "1234"
        )));
    }

    #[test]
    fn test_does_not_affect_non_200() {
        let code = (302, "Found");
        let handler = returning!(code, "Last-Modified" => before_now(), "ETag" => "1234");
        expect(
            code,
            handler.call(&mut request!(
                "If-Modified-Since" => now(),
                "If-None-Match" => "1234"
            )),
        );
    }

    #[test]
    fn test_does_not_affect_malformed_timestamp() {
        let bad_stamp = time::now()
            .strftime("%Y-%m-%d %H:%M:%S %z")
            .unwrap()
            .to_string();
        let handler = returning!("Last-Modified" => before_now());
        expect_200(handler.call(&mut request!(
            "If-Modified-Since" => bad_stamp
        )));
    }

    fn expect_304(response: Result<Response, Box<dyn Error + Send>>) {
        let mut response = response.ok().expect("No response");
        let mut body = Vec::new();
        response.body.write_body(&mut body).ok().expect("No body");

        assert_eq!(response.status, (304, "Not Modified"));
        assert_eq!(body, b"");
    }

    fn expect_200(response: Result<Response, Box<dyn Error + Send>>) {
        expect((200, "OK"), response);
    }

    fn expect(status: (u32, &'static str), response: Result<Response, Box<dyn Error + Send>>) {
        let mut response = response.ok().expect("No response");
        let mut body = Vec::new();
        response.body.write_body(&mut body).ok().expect("No body");

        assert_eq!(response.status, status);
        assert_eq!(body, b"hello");
    }

    struct SimpleHandler {
        map: HashMap<String, Vec<String>>,
        status: Status,
        body: &'static str,
    }

    type Status = (u32, &'static str);

    impl SimpleHandler {
        fn new(
            map: HashMap<String, Vec<String>>,
            status: Status,
            body: &'static str,
        ) -> SimpleHandler {
            SimpleHandler {
                map: map,
                status: status,
                body: body,
            }
        }
    }

    impl Handler for SimpleHandler {
        fn call(&self, _: &mut dyn Request) -> Result<Response, Box<dyn Error + Send>> {
            Ok(Response {
                status: self.status,
                headers: self.map.clone(),
                body: Box::new(Cursor::new(self.body.to_string().into_bytes())),
            })
        }
    }

    fn before_now() -> String {
        let mut now = time::now();
        now.tm_year -= 1;
        httpdate(now)
    }

    fn now() -> String {
        httpdate(time::now())
    }

    fn httpdate(time: Tm) -> String {
        time.strftime("%a, %d-%m-%y %T GMT").unwrap().to_string()
    }
}
