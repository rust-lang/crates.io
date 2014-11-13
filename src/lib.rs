#![feature(macro_rules)]

extern crate time;
extern crate conduit;
extern crate "conduit-middleware" as middleware;

use std::str::{MaybeOwned, Slice, Owned};
use std::fmt::Show;
use std::io::util::NullReader;
use time::{Tm, strptime, ParseError};
use conduit::Request;
use middleware::Middleware;

type Response = Result<conduit::Response, Box<Show + 'static>>;

pub struct ConditionalGet;

impl Middleware for ConditionalGet {
    fn after(&self, req: &mut Request, res: Response) -> Response {
        let mut res = try!(res);

        match req.method() {
            conduit::Get | conduit::Head => {
                if is_ok(&res) && is_fresh(req, &res) {
                    res.status = (304, "Not Modified");
                    res.headers.pop_equiv("Content-Type");
                    res.headers.pop_equiv("Content-Length");
                    res.body = box NullReader as Box<Reader + Send>;
                }
            },
            _ => ()
        }

        Ok(res)
    }
}

fn is_ok(response: &conduit::Response) -> bool {
    match *response {
        conduit::Response { status: (200, _), .. } => true,
        _ => false
    }
}

fn is_fresh(req: &Request, res: &conduit::Response) -> bool {
    let modified_since = req.headers().find("If-Modified-Since").map(header_val);
    let none_match     = req.headers().find("If-None-Match").map(header_val);

    if modified_since.as_ref().or_else(|| none_match.as_ref()).is_none() {
        return false;
    }

    let mut success = true;

    modified_since.and_then(|modified_since| {
        parse_http_date(modified_since).map_err(|_| { success = false; }).ok()
    }).map(|parsed| {
        success = success && is_modified_since(parsed, res);
    });

    none_match.map(|none_match| {
        success = success && etag_matches(none_match, res);
    });

    success
}

fn etag_matches<S: Str>(none_match: S, res: &conduit::Response) -> bool {
    res.headers.find_equiv("ETag").map(|etag| {
        res_header_val(etag).as_slice() == none_match.as_slice()
    }).unwrap_or(false)
}

fn is_modified_since(modified_since: Tm, res: &conduit::Response) -> bool {
    res.headers.find_equiv("Last-Modified").and_then(|last_modified| {
        parse_http_date(res_header_val(last_modified).as_slice()).ok()
    }).map(|last_modified| {
        modified_since.to_timespec() >= last_modified.to_timespec()
    }).unwrap_or(false)
}

fn header_val<'a>(header: Vec<&'a str>) -> MaybeOwned<'a> {
    if header.len() == 1 {
        Slice(header[0])
    } else {
        Owned(header.concat())
    }
}

fn res_header_val<'a>(header: &'a Vec<String>) -> MaybeOwned<'a> {
    if header.len() == 1 {
        Slice(header[0].as_slice())
    } else {
        Owned(header.concat())
    }
}

fn parse_http_date<S: Str>(string: S) -> Result<Tm, ()> {
    let string = string.as_slice();

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
    extern crate "conduit-test" as test;

    use std::fmt::Show;
    use std::io::MemReader;
    use std::collections::HashMap;
    use time;
    use time::Tm;
    use conduit;
    use conduit::{Request, Response, Handler};
    use middleware::MiddlewareBuilder;

    use super::ConditionalGet;

    macro_rules! returning(
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
    )

    macro_rules! request(
        ($($header:expr => $value:expr),+) => ({
            let mut req = test::MockRequest::new(conduit::Get, "/");
            $(req.header($header, $value.to_string());)+
            req
        })
    )

    #[test]
    fn test_sends_304() {
        let handler = returning!("Last-Modified" => httpdate(time::now()));
        expect_304(handler.call(&mut request!(
            "If-Modified-Since" => httpdate(time::now())
        ) as &mut conduit::Request));
    }

    #[test]
    fn test_sends_304_if_older_than_now() {
        let handler = returning!("Last-Modified" => before_now());
        expect_304(handler.call(&mut request!(
            "If-Modified-Since" => httpdate(time::now())
        ) as &mut conduit::Request));
    }

    #[test]
    fn test_sends_304_with_etag() {
        let handler = returning!("ETag" => "1234");
        expect_304(handler.call(&mut request!(
            "If-None-Match" => "1234"
        ) as &mut conduit::Request));
    }

    #[test]
    fn test_sends_200_with_fresh_time_but_not_etag() {
        let handler = returning!("Last-Modified" => before_now(), "ETag" => "1234");
        expect_200(handler.call(&mut request!(
            "If-Modified-Since" => now(),
            "If-None-Match" => "4321"
        ) as &mut conduit::Request));
    }

    #[test]
    fn test_sends_200_with_fresh_etag_but_not_time() {
        let handler = returning!("Last-Modified" => now(), "ETag" => "1234");
        expect_200(handler.call(&mut request!(
            "If-Modified-Since" => before_now(),
            "If-None-Match" => "1234"
        ) as &mut conduit::Request));
    }

    #[test]
    fn test_sends_200_with_fresh_etag() {
        let handler = returning!("ETag" => "1234");
        expect_200(handler.call(&mut request!(
            "If-None-Match" => "4321"
        ) as &mut conduit::Request));
    }

    #[test]
    fn test_sends_200_with_fresh_time() {
        let handler = returning!("Last-Modified" => now());
        expect_200(handler.call(&mut request!(
            "If-Modified-Since" => before_now()
        ) as &mut conduit::Request));
    }

    #[test]
    fn test_sends_304_with_fresh_time_and_etag() {
        let handler = returning!("Last-Modified" => before_now(), "ETag" => "1234");
        expect_304(handler.call(&mut request!(
            "If-Modified-Since" => now(),
            "If-None-Match" => "1234"
        ) as &mut conduit::Request));
    }

    #[test]
    fn test_does_not_affect_non_200() {
        let code = (302, "Found");
        let handler = returning!(code, "Last-Modified" => before_now(), "ETag" => "1234");
        expect(code, handler.call(&mut request!(
            "If-Modified-Since" => now(),
            "If-None-Match" => "1234"
        ) as &mut conduit::Request));
    }

    #[test]
    fn test_does_not_affect_malformed_timestamp() {
        let bad_stamp = time::now().strftime("%Y-%m-%d %H:%M:%S %z").unwrap().to_string();
        let handler = returning!("Last-Modified" => before_now());
        expect_200(handler.call(&mut request!(
            "If-Modified-Since" => bad_stamp
        ) as &mut conduit::Request));
    }

    fn expect_304(response: Result<Response, Box<Show>>) {
        let mut response = response.ok().expect("No response");
        let body = response.body.read_to_string().ok().expect("No body");

        assert_eq!(response.status, (304, "Not Modified"));
        assert_eq!(body.as_slice(), "");
    }

    fn expect_200(response: Result<Response, Box<Show>>) {
        expect((200, "OK"), response);
    }

    fn expect(status: (uint, &'static str), response: Result<Response, Box<Show>>) {
        let mut response = response.ok().expect("No response");
        let body = response.body.read_to_string().ok().expect("No body");

        assert_eq!(response.status, status);
        assert_eq!(body.as_slice(), "hello");
    }

    struct SimpleHandler {
        map: HashMap<String, Vec<String>>,
        status: Status,
        body: &'static str
    }

    type Status = (uint, &'static str);

    impl SimpleHandler {
        fn new(map: HashMap<String, Vec<String>>, status: Status, body: &'static str) -> SimpleHandler {
            SimpleHandler { map: map, status: status, body: body }
        }
    }

    impl Handler for SimpleHandler {
        fn call(&self, _: &mut Request) -> Result<Response, Box<Show + 'static>> {
            Ok(Response {
                status: self.status,
                headers: self.map.clone(),
                body: box MemReader::new(self.body.to_string().into_bytes()) as Box<Reader + Send>
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
