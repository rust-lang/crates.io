use conduit::RequestExt;
use http::header::AsHeaderName;
use http::HeaderMap;

/// Returns the value of the request header, or an empty slice if it is not
/// present.
///
/// If a header appears multiple times, this will return only one of them.
///
/// If the header value is invalid utf8, an empty slice will be returned.
pub fn request_header<K>(req: &dyn RequestExt, key: K) -> &str
where
    K: AsHeaderName,
{
    req.headers().get_str_or_default(key)
}

pub trait HeaderMapExt {
    /// Returns the value of the request header, or an empty slice if it is not
    /// present.
    ///
    /// If a header appears multiple times, this will return only one of them.
    ///
    /// If the header value is invalid utf8, an empty slice will be returned.
    fn get_str_or_default<K: AsHeaderName>(&self, key: K) -> &str;
}

impl HeaderMapExt for HeaderMap {
    fn get_str_or_default<K: AsHeaderName>(&self, key: K) -> &str {
        self.get(key)
            .map(|value| value.to_str().unwrap_or_default())
            .unwrap_or_default()
    }
}
