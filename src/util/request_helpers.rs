use crate::controllers::util::RequestPartsExt;
use axum::response::{IntoResponse, Response};
use axum_extra::TypedHeader;
use axum_extra::headers::CacheControl;
use http::header::AsHeaderName;
use http::{HeaderMap, StatusCode, header};
use indexmap::IndexMap;

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

pub fn redirect(url: String) -> Response {
    (StatusCode::FOUND, [(header::LOCATION, url)]).into_response()
}

/// Build a `Cache-Control: no-store` header that prevents any cache (shared CDN
/// cache or browser cache) from storing a per-user response.
///
/// Used on responses that depend on the authenticated identity, which can come
/// from a session cookie or an API token.
pub fn no_store() -> TypedHeader<CacheControl> {
    TypedHeader(CacheControl::new().with_no_store())
}

pub trait RequestUtils {
    fn wants_json(&self) -> bool;
    fn query_with_params(&self, params: IndexMap<String, String>) -> String;
}

impl<T: RequestPartsExt> RequestUtils for T {
    fn wants_json(&self) -> bool {
        self.headers()
            .get_all(header::ACCEPT)
            .iter()
            .any(|val| val.to_str().unwrap_or_default().contains("json"))
    }

    fn query_with_params(&self, new_params: IndexMap<String, String>) -> String {
        let query_bytes = self.uri().query().unwrap_or("").as_bytes();

        let mut params = url::form_urlencoded::parse(query_bytes)
            .into_owned()
            .filter(|(key, _)| !new_params.contains_key(key))
            .collect::<Vec<_>>();

        params.extend(new_params);

        let query_string = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(params)
            .finish();
        format!("?{query_string}")
    }
}
