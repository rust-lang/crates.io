use serde_json::Value;
use std::marker::PhantomData;
use std::ops::Deref;

use crates_io::rate_limiter::LimitedAction;
use http::{header, StatusCode};

/// A type providing helper methods for working with responses
#[must_use]
pub struct Response<T> {
    response: reqwest::blocking::Response,
    return_type: PhantomData<T>,
}

impl<T> Response<T>
where
    for<'de> T: serde::Deserialize<'de>,
{
    /// Assert that the response is good and deserialize the message
    #[track_caller]
    pub fn good(self) -> T {
        if !self.status().is_success() {
            panic!("bad response: {:?}", self.status());
        }
        json(self.response)
    }
}

impl<T> Response<T> {
    #[track_caller]
    pub(super) fn new(response: reqwest::blocking::Response) -> Self {
        Self {
            response,
            return_type: PhantomData,
        }
    }

    /// Consume the response body and convert it to a JSON value
    #[track_caller]
    pub fn into_json(self) -> Value {
        json(self.response)
    }

    #[track_caller]
    pub fn into_text(self) -> String {
        assert_ok!(self.response.text())
    }

    #[track_caller]
    pub fn assert_redirect_ends_with(&self, target: &str) -> &Self {
        assert!(self
            .response
            .headers()
            .get(header::LOCATION)
            .unwrap()
            .to_str()
            .unwrap()
            .ends_with(target));
        self
    }

    /// Assert that the status code is 429 and that the body matches a rate limit.
    #[track_caller]
    pub fn assert_rate_limited(self, action: LimitedAction) {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct ErrorResponse {
            errors: Vec<ErrorDetails>,
        }
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct ErrorDetails {
            detail: String,
        }

        assert_eq!(StatusCode::TOO_MANY_REQUESTS, self.status());

        let expected_message_start = format!("{}. Please try again after ", action.error_message());
        let error: ErrorResponse = json(self.response);
        assert_eq!(1, error.errors.len());
        assert!(error.errors[0].detail.starts_with(&expected_message_start));
    }
}

impl Response<()> {
    /// Assert that the status code is 404
    #[track_caller]
    pub fn assert_not_found(&self) {
        assert_eq!(StatusCode::NOT_FOUND, self.status());
    }

    /// Assert that the status code is 403
    #[track_caller]
    pub fn assert_forbidden(&self) {
        assert_eq!(StatusCode::FORBIDDEN, self.status());
    }
}

impl<T> Deref for Response<T> {
    type Target = reqwest::blocking::Response;

    fn deref(&self) -> &Self::Target {
        &self.response
    }
}

fn json<T>(r: reqwest::blocking::Response) -> T
where
    for<'de> T: serde::Deserialize<'de>,
{
    let content_type = r
        .headers()
        .get(header::CONTENT_TYPE)
        .expect("Missing content-type header");

    assert_eq!(content_type, "application/json");

    let content_length: usize = r
        .headers()
        .get(header::CONTENT_LENGTH)
        .expect("Missing content-length header")
        .to_str()
        .unwrap()
        .parse()
        .unwrap();

    let bytes = r.bytes().unwrap();
    assert_eq!(content_length, bytes.len());

    match serde_json::from_slice(&bytes) {
        Ok(t) => t,
        Err(e) => panic!("failed to decode: {e:?}"),
    }
}
