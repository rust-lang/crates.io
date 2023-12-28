use crate::config::Server;
use crate::controllers::prelude::*;
use crate::controllers::util::RequestPartsExt;
use crate::middleware::log_request::RequestLogExt;
use crate::middleware::real_ip::RealIp;
use crate::models::helpers::with_count::*;
use crate::util::errors::{bad_request, AppResult};
use crate::util::HeaderMapExt;

use base64::{engine::general_purpose, Engine};
use diesel::pg::Pg;
use diesel::query_builder::*;
use diesel::query_dsl::LoadQuery;
use diesel::sql_types::BigInt;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

const MAX_PAGE_BEFORE_SUSPECTED_BOT: u32 = 10;
const DEFAULT_PER_PAGE: i64 = 10;
const MAX_PER_PAGE: i64 = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Page {
    Numeric(u32),
    Seek(RawSeekPayload),
    Unspecified,
}

#[derive(Debug, Clone)]
pub(crate) struct PaginationOptions {
    pub(crate) page: Page,
    pub(crate) per_page: i64,
}

impl PaginationOptions {
    pub(crate) fn builder() -> PaginationOptionsBuilder {
        PaginationOptionsBuilder {
            limit_page_numbers: false,
            enable_seek: false,
            enable_pages: true,
        }
    }

    pub(crate) fn offset(&self) -> Option<i64> {
        if let Page::Numeric(p) = self.page {
            Some((p - 1) as i64 * self.per_page)
        } else {
            None
        }
    }
}

pub(crate) struct PaginationOptionsBuilder {
    limit_page_numbers: bool,
    enable_pages: bool,
    enable_seek: bool,
}

impl PaginationOptionsBuilder {
    pub(crate) fn limit_page_numbers(mut self) -> Self {
        self.limit_page_numbers = true;
        self
    }

    pub(crate) fn enable_pages(mut self, enable: bool) -> Self {
        self.enable_pages = enable;
        self
    }

    pub(crate) fn enable_seek(mut self, enable: bool) -> Self {
        self.enable_seek = enable;
        self
    }

    pub(crate) fn gather<T: RequestPartsExt>(self, req: &T) -> AppResult<PaginationOptions> {
        let params = req.query();
        let page_param = params.get("page");
        let seek_param = params.get("seek");

        if seek_param.is_some() && page_param.is_some() {
            return Err(bad_request(
                "providing both ?page= and ?seek= is unsupported",
            ));
        }

        let page = if let Some(s) = page_param {
            if self.enable_pages {
                let numeric_page = s.parse().map_err(bad_request)?;
                if numeric_page < 1 {
                    return Err(bad_request(format_args!(
                        "page indexing starts from 1, page {numeric_page} is invalid",
                    )));
                }

                if numeric_page > MAX_PAGE_BEFORE_SUSPECTED_BOT {
                    req.request_log().add("bot", "suspected");
                }

                // Block large offsets for known violators of the crawler policy
                if self.limit_page_numbers {
                    let config = &req.app().config;
                    if numeric_page > config.max_allowed_page_offset
                        && is_useragent_or_ip_blocked(config, req)
                    {
                        req.request_log().add("cause", "large page offset");

                        let error =
                            format!("Page {numeric_page} is unavailable for performance reasons. Please take a look at https://crates.io/data-access for alternatives.");

                        return Err(bad_request(error));
                    }
                }

                Page::Numeric(numeric_page)
            } else {
                return Err(bad_request("?page= is not supported for this request"));
            }
        } else if let Some(s) = seek_param {
            if self.enable_seek {
                Page::Seek(RawSeekPayload(s.clone()))
            } else {
                return Err(bad_request("?seek= is not supported for this request"));
            }
        } else {
            Page::Unspecified
        };

        let per_page = params
            .get("per_page")
            .map(|s| s.parse().map_err(bad_request))
            .unwrap_or(Ok(DEFAULT_PER_PAGE))?;
        if per_page > MAX_PER_PAGE {
            return Err(bad_request(format_args!(
                "cannot request more than {MAX_PER_PAGE} items",
            )));
        }

        Ok(PaginationOptions { page, per_page })
    }
}

pub(crate) trait Paginate: Sized {
    fn pages_pagination(self, options: PaginationOptions) -> PaginatedQuery<Self> {
        PaginatedQuery {
            query: self,
            options,
        }
    }
}

impl<T> Paginate for T {}

pub struct Paginated<T> {
    records_and_total: Vec<WithCount<T>>,
    options: PaginationOptions,
}

impl<T> Paginated<T> {
    pub(crate) fn total(&self) -> i64 {
        self.records_and_total
            .first()
            .map(|row| row.total)
            .unwrap_or_default() // If there is no first row, then the total is zero.
    }

    pub(crate) fn next_page_params(&self) -> Option<IndexMap<String, String>> {
        if self.records_and_total.len() < self.options.per_page as usize {
            return None;
        }

        let mut opts = IndexMap::new();
        match self.options.page {
            Page::Numeric(n) => opts.insert("page".into(), (n + 1).to_string()),
            Page::Unspecified => opts.insert("page".into(), 2.to_string()),
            Page::Seek(_) => return None,
        };
        Some(opts)
    }

    pub(crate) fn prev_page_params(&self) -> Option<IndexMap<String, String>> {
        let mut opts = IndexMap::new();
        match self.options.page {
            Page::Numeric(1) | Page::Unspecified | Page::Seek(_) => return None,
            Page::Numeric(n) => opts.insert("page".into(), (n - 1).to_string()),
        };
        Some(opts)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.records_and_total.iter().map(|row| &row.record)
    }
}

impl<T: 'static> IntoIterator for Paginated<T> {
    type IntoIter = Box<dyn Iterator<Item = Self::Item>>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.records_and_total.into_iter().map(|row| row.record))
    }
}

#[derive(Debug)]
pub(crate) struct PaginatedQuery<T> {
    query: T,
    options: PaginationOptions,
}

impl<T> PaginatedQuery<T> {
    pub(crate) fn load<'a, U>(self, conn: &mut PgConnection) -> QueryResult<Paginated<U>>
    where
        Self: LoadQuery<'a, PgConnection, WithCount<U>>,
    {
        let options = self.options.clone();
        let records_and_total = self.internal_load(conn)?.collect::<QueryResult<_>>()?;
        Ok(Paginated {
            records_and_total,
            options,
        })
    }
}

impl<T> QueryId for PaginatedQuery<T> {
    const HAS_STATIC_QUERY_ID: bool = false;
    type QueryId = ();
}

impl<T: Query> Query for PaginatedQuery<T> {
    type SqlType = (T::SqlType, BigInt);
}

impl<T, DB> RunQueryDsl<DB> for PaginatedQuery<T> {}

impl<T> QueryFragment<Pg> for PaginatedQuery<T>
where
    T: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_sql("SELECT *, COUNT(*) OVER () FROM (");
        self.query.walk_ast(out.reborrow())?;
        out.push_sql(") t LIMIT ");
        out.push_bind_param::<BigInt, _>(&self.options.per_page)?;
        if let Some(offset) = self.options.offset() {
            out.push_sql(format!(" OFFSET {offset}").as_str());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawSeekPayload(String);

impl RawSeekPayload {
    pub(crate) fn decode<D: for<'a> Deserialize<'a>>(&self) -> AppResult<D> {
        decode_seek(&self.0).map_err(|_| bad_request("invalid seek parameter"))
    }
}

/// Function to check if the request is blocked.
///
/// A request can be blocked if either the User Agent is on the User Agent block list or if the client
/// IP is on the CIDR block list.
fn is_useragent_or_ip_blocked<T: RequestPartsExt>(config: &Server, req: &T) -> bool {
    let user_agent = req.headers().get_str_or_default(header::USER_AGENT);
    let client_ip = req.extensions().get::<RealIp>();

    // check if user agent is blocked
    if config
        .page_offset_ua_blocklist
        .iter()
        .any(|blocked| user_agent.contains(blocked))
    {
        return true;
    }

    // check if client ip is blocked, needs to be an IPv4 address
    if let Some(client_ip) = client_ip {
        if config
            .page_offset_cidr_blocklist
            .iter()
            .any(|blocked| blocked.contains(**client_ip))
        {
            return true;
        }
    }

    false
}

/// Encode a payload to be used as a seek key.
///
/// The payload is base64-encoded to hint that it shouldn't be manually constructed. There is no
/// technical measure to prevent API consumers for manually creating or modifying them, but
/// hopefully the base64 will be enough to convey that doing it is unsupported.
pub(crate) fn encode_seek<S: Serialize>(params: S) -> AppResult<String> {
    let encoded = general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(&params)?);
    Ok(encoded)
}

/// Decode a list of params previously encoded with [`encode_seek`].
pub(crate) fn decode_seek<D: for<'a> Deserialize<'a>>(seek: &str) -> anyhow::Result<D> {
    let decoded = serde_json::from_slice(&general_purpose::URL_SAFE_NO_PAD.decode(seek)?)?;
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::{Method, StatusCode};

    #[test]
    fn no_pagination_param() {
        let pagination = PaginationOptions::builder().gather(&mock("")).unwrap();
        assert_eq!(Page::Unspecified, pagination.page);
        assert_eq!(DEFAULT_PER_PAGE, pagination.per_page);
    }

    #[test]
    fn page_param_parsing() {
        let assert_error =
            |query, msg| assert_pagination_error(PaginationOptions::builder(), query, msg);

        assert_error("page=", "cannot parse integer from empty string");
        assert_error("page=not_a_number", "invalid digit found in string");
        assert_error("page=1.0", "invalid digit found in string");
        assert_error("page=0", "page indexing starts from 1, page 0 is invalid");

        let pagination = PaginationOptions::builder()
            .gather(&mock("page=5"))
            .unwrap();
        assert_eq!(Page::Numeric(5), pagination.page);
    }

    #[test]
    fn per_page_param_parsing() {
        let assert_error =
            |query, msg| assert_pagination_error(PaginationOptions::builder(), query, msg);

        assert_error("per_page=", "cannot parse integer from empty string");
        assert_error("per_page=not_a_number", "invalid digit found in string");
        assert_error("per_page=1.0", "invalid digit found in string");
        assert_error("per_page=101", "cannot request more than 100 items");

        let pagination = PaginationOptions::builder()
            .gather(&mock("per_page=5"))
            .unwrap();
        assert_eq!(pagination.per_page, 5);
    }

    #[test]
    fn seek_param_parsing() {
        assert_pagination_error(
            PaginationOptions::builder(),
            "seek=OTg",
            "?seek= is not supported for this request",
        );

        let pagination = PaginationOptions::builder()
            .enable_seek(true)
            .gather(&mock("seek=OTg"))
            .unwrap();

        if let Page::Seek(raw) = pagination.page {
            assert_ok_eq!(raw.decode::<i32>(), 98);
        } else {
            panic!(
                "did not parse a seek page, parsed {:?} instead",
                pagination.page
            );
        }
    }

    #[test]
    fn both_page_and_seek() {
        assert_pagination_error(
            PaginationOptions::builder(),
            "page=1&seek=OTg",
            "providing both ?page= and ?seek= is unsupported",
        );
        assert_pagination_error(
            PaginationOptions::builder().enable_seek(true),
            "page=1&seek=OTg",
            "providing both ?page= and ?seek= is unsupported",
        );
    }

    #[test]
    fn disabled_pages() {
        assert_pagination_error(
            PaginationOptions::builder().enable_pages(false),
            "page=1",
            "?page= is not supported for this request",
        );
    }

    #[test]
    fn test_seek_encode_and_decode() {
        // Encoding produces the results we expect
        assert_ok_eq!(encode_seek(98), "OTg");
        assert_ok_eq!(encode_seek(("foo", 42)), "WyJmb28iLDQyXQ");

        // Encoded values can be then decoded.
        assert_ok_eq!(decode_seek::<i32>(&encode_seek(98).unwrap()), 98);
        assert_ok_eq!(
            decode_seek::<(String, i32)>(&encode_seek(("foo", 42)).unwrap()),
            ("foo".into(), 42),
        );
    }

    fn mock(query: &str) -> Request<()> {
        Request::builder()
            .method(Method::GET)
            .uri(format!("/?{query}"))
            .body(())
            .unwrap()
    }

    fn assert_pagination_error(options: PaginationOptionsBuilder, query: &str, message: &str) {
        let error = options.gather(&mock(query)).unwrap_err();
        assert_eq!(error.to_string(), message);

        let response = error.response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
