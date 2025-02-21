use crate::tests::util::matchers::is_success;
use bytes::Bytes;
use googletest::prelude::*;
use serde_json::Value;
use std::marker::PhantomData;
use std::str::from_utf8;

use crate::rate_limiter::LimitedAction;
use derive_more::Deref;
use http::{StatusCode, header};

/// A type providing helper methods for working with responses
#[derive(Deref)]
#[must_use]
pub struct Response<T> {
    #[deref]
    response: hyper::Response<Bytes>,
    return_type: PhantomData<T>,
}

impl<T> Response<T>
where
    for<'de> T: serde::Deserialize<'de>,
{
    /// Assert that the response is good and deserialize the message
    #[track_caller]
    pub fn good(self) -> T {
        assert_that!(self.status(), is_success());
        json(&self.response)
    }
}

impl<T> Response<T> {
    #[track_caller]
    pub(super) fn new(response: hyper::Response<Bytes>) -> Self {
        Self {
            response,
            return_type: PhantomData,
        }
    }

    /// Consume the response body and convert it to a JSON value
    #[track_caller]
    pub fn json(&self) -> Value {
        json(&self.response)
    }

    #[track_caller]
    pub fn text(&self) -> String {
        let bytes = self.response.body();
        assert_ok!(from_utf8(bytes)).to_string()
    }

    #[track_caller]
    pub fn assert_redirect_ends_with(&self, target: &str) -> &Self {
        let headers = self.response.headers();
        let location = assert_some!(headers.get(header::LOCATION));
        let location = assert_ok!(location.to_str());
        assert_that!(location, ends_with(target));
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
        #[derive(Debug, serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct ErrorDetails {
            detail: String,
        }

        assert_eq!(self.status(), StatusCode::TOO_MANY_REQUESTS);

        let expected_message_start = format!("{}. Please try again after ", action.error_message());
        let error: ErrorResponse = json(&self.response);
        assert_that!(error.errors, len(eq(1)));
        assert_that!(error.errors[0].detail, starts_with(expected_message_start));
    }
}

impl Response<()> {
    /// Assert that the status code is 404
    #[track_caller]
    pub fn assert_not_found(&self) {
        assert_eq!(self.status(), StatusCode::NOT_FOUND);
    }

    /// Assert that the status code is 403
    #[track_caller]
    pub fn assert_forbidden(&self) {
        assert_eq!(self.status(), StatusCode::FORBIDDEN);
    }
}

fn json<T>(r: &hyper::Response<Bytes>) -> T
where
    for<'de> T: serde::Deserialize<'de>,
{
    fn inner(r: &hyper::Response<Bytes>) -> &Bytes {
        let headers = r.headers();

        assert_some_eq!(headers.get(header::CONTENT_TYPE), "application/json");

        let content_length = assert_some!(
            r.headers().get(header::CONTENT_LENGTH),
            "Missing content-length header"
        );
        let content_length = assert_ok!(content_length.to_str());
        let content_length: usize = assert_ok!(content_length.parse());

        let bytes = r.body();
        assert_that!(*bytes, len(eq(content_length)));

        bytes
    }

    match serde_json::from_slice(inner(r)) {
        Ok(t) => t,
        Err(e) => panic!("failed to decode: {e:?}"),
    }
}
