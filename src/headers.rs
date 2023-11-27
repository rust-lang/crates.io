use axum_extra::headers::{Error, Header};
use http::header::{HeaderName, HeaderValue};

static X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

pub struct XRequestId(String);

impl XRequestId {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Header for XRequestId {
    fn name() -> &'static HeaderName {
        &X_REQUEST_ID
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        values
            .next()
            .and_then(|value| value.to_str().ok())
            .map(|value| Self(value.to_string()))
            .ok_or_else(Error::invalid)
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        let value = HeaderValue::from_str(&self.0).unwrap();
        values.extend(std::iter::once(value));
    }
}
