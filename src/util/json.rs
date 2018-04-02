use std::collections::HashMap;
use std::io::Cursor;

use serde_json;
use serde::Serialize;

use conduit::Response;

pub fn json_response<T: Serialize>(t: &T) -> Response {
    let json = serde_json::to_string(t).unwrap();
    let mut headers = HashMap::new();
    headers.insert(
        "Content-Type".to_string(),
        vec!["application/json; charset=utf-8".to_string()],
    );
    headers.insert("Content-Length".to_string(), vec![json.len().to_string()]);
    Response {
        status: (200, "OK"),
        headers,
        body: Box::new(Cursor::new(json.into_bytes())),
    }
}

#[derive(Serialize)]
struct StringError<'a> {
    detail: &'a str,
}

#[derive(Serialize)]
struct Bad<'a> {
    errors: Vec<StringError<'a>>,
}

pub fn json_error(status: (u32, &'static str), detail: &str) -> Response {
    let mut response = json_error_200(detail);
    response.status = status;
    response
}

pub fn json_error_200(detail: &str) -> Response {
    json_response(&Bad {
        errors: vec![StringError { detail }],
    })
}
