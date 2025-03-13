use crate::config::Server;
use crate::middleware::app::RequestApp;
use crate::middleware::log_request::RequestLogExt;
use crate::middleware::real_ip::RealIp;
use crate::models::helpers::with_count::*;
use crate::util::HeaderMapExt;
use crate::util::errors::{AppResult, bad_request};
use std::num::NonZeroU32;

use axum::extract::FromRequestParts;
use base64::{Engine, engine::general_purpose};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::query_builder::{AstPass, Query, QueryFragment, QueryId};
use diesel::sql_types::BigInt;
use diesel_async::AsyncPgConnection;
use futures_util::future::BoxFuture;
use futures_util::{FutureExt, TryStreamExt};
use http::header;
use http::request::Parts;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

const MAX_PAGE_BEFORE_SUSPECTED_BOT: u32 = 10;
const DEFAULT_PER_PAGE: i64 = 10;
const MAX_PER_PAGE: i64 = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Page {
    Numeric(u32),
    Seek(RawSeekPayload),
    SeekBackward(RawSeekPayload),
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
            enable_seek_backward: false,
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

    pub(crate) fn is_backward(&self) -> bool {
        matches!(self.page, Page::SeekBackward(_))
    }

    pub(crate) fn is_explicit(&self) -> bool {
        matches!(self.page, Page::Numeric(_))
    }
}

#[derive(Debug, Deserialize, FromRequestParts, utoipa::IntoParams)]
#[from_request(via(axum::extract::Query))]
#[into_params(parameter_in = Query)]
pub struct PaginationQueryParams {
    /// The page number to request.
    ///
    /// This parameter is mutually exclusive with `seek` and not supported for
    /// all requests.
    #[param(value_type = Option<u32>, minimum = 1)]
    pub page: Option<NonZeroU32>,

    /// The number of items to request per page.
    #[param(value_type = Option<u32>, minimum = 1)]
    pub per_page: Option<NonZeroU32>,

    /// The seek key to request.
    ///
    /// This parameter is mutually exclusive with `page` and not supported for
    /// all requests.
    ///
    /// The seek key can usually be found in the `meta.next_page` field of
    /// paginated responses. It can also be found in the `meta.prev_page` field
    /// when the endpoint supports backward pagination, in which case the value
    /// starts with a `-`.
    pub seek: Option<String>,
}

pub(crate) struct PaginationOptionsBuilder {
    limit_page_numbers: bool,
    enable_pages: bool,
    enable_seek: bool,
    enable_seek_backward: bool,
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

    #[allow(dead_code)]
    pub(crate) fn enable_seek_backward(mut self, enable: bool) -> Self {
        self.enable_seek_backward = enable;
        self
    }

    pub(crate) fn gather(self, parts: &Parts) -> AppResult<PaginationOptions> {
        use axum::extract::Query;

        let Query(params) = Query::<PaginationQueryParams>::try_from_uri(&parts.uri)
            .map_err(|err| bad_request(err.body_text()))?;

        if params.seek.is_some() && params.page.is_some() {
            return Err(bad_request(
                "providing both ?page= and ?seek= is unsupported",
            ));
        }

        let page = if let Some(s) = params.page {
            if !self.enable_pages {
                return Err(bad_request("?page= is not supported for this request"));
            }

            let numeric_page = s.get();
            if numeric_page > MAX_PAGE_BEFORE_SUSPECTED_BOT {
                parts.request_log().add("bot", "suspected");
            }

            // Block large offsets for known violators of the crawler policy
            if self.limit_page_numbers {
                let config = &parts.app().config;
                if numeric_page > config.max_allowed_page_offset
                    && is_useragent_or_ip_blocked(config, parts)
                {
                    parts.request_log().add("cause", "large page offset");

                    let error = format!(
                        "Page {numeric_page} is unavailable for performance reasons. Please take a look at https://crates.io/data-access for alternatives."
                    );

                    return Err(bad_request(error));
                }
            }

            Page::Numeric(numeric_page)
        } else if let Some(s) = params.seek {
            match s.starts_with('-') {
                true if !self.enable_seek_backward => {
                    return Err(bad_request(
                        "seek backward ?seek=- is not supported for this request",
                    ));
                }
                true => Page::SeekBackward(RawSeekPayload(s.trim_start_matches('-').into())),
                false if !self.enable_seek => {
                    return Err(bad_request("?seek= is not supported for this request"));
                }
                false => Page::Seek(RawSeekPayload(s)),
            }
        } else {
            Page::Unspecified
        };

        let per_page = params.per_page.map(|p| p.get() as i64);
        let per_page = per_page.unwrap_or(DEFAULT_PER_PAGE);
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

    fn pages_pagination_with_count_query<C>(
        self,
        options: PaginationOptions,
        count_query: C,
    ) -> PaginatedQueryWithCountSubq<Self, C> {
        PaginatedQueryWithCountSubq {
            query: self,
            count_query,
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
            Page::Seek(_) | Page::SeekBackward(_) => return None,
        };
        Some(opts)
    }

    pub(crate) fn prev_page_params(&self) -> Option<IndexMap<String, String>> {
        let mut opts = IndexMap::new();
        match self.options.page {
            Page::Numeric(1) | Page::Unspecified | Page::Seek(_) | Page::SeekBackward(_) => {
                return None;
            }
            Page::Numeric(n) => opts.insert("page".into(), (n - 1).to_string()),
        };
        Some(opts)
    }

    pub(crate) fn next_seek_params<S, F>(&self, f: F) -> AppResult<Option<IndexMap<String, String>>>
    where
        F: Fn(&T) -> S,
        S: Serialize,
    {
        // When the data size is smaller than the page size, we would expect the next page to be
        // available during backward pagination but unavailable during forward pagination.
        if self.options.is_explicit()
            || self.records_and_total.is_empty()
            || (self.records_and_total.len() < self.options.per_page as usize
                && !self.options.is_backward())
        {
            return Ok(None);
        }

        // We also like to return None for next page when it's the first backward pagination.
        let mut opts = IndexMap::new();
        match self.options.page {
            Page::SeekBackward(ref raw) if raw.is_empty() => return Ok(None),
            Page::Unspecified | Page::Seek(_) | Page::SeekBackward(_) => {
                let seek = f(&self.records_and_total.last().unwrap().record);
                opts.insert("seek".into(), encode_seek(seek)?);
            }
            Page::Numeric(_) => unreachable!(),
        };
        Ok(Some(opts))
    }

    pub(crate) fn prev_seek_params<S, F>(&self, f: F) -> AppResult<Option<IndexMap<String, String>>>
    where
        F: Fn(&T) -> S,
        S: Serialize,
    {
        // When the data size is smaller than the page size, we would expect the prev page to be
        // unavailable during backward pagination but available during forward pagination.
        if self.options.is_explicit()
            || self.records_and_total.is_empty()
            || (self.records_and_total.len() < self.options.per_page as usize
                && self.options.is_backward())
        {
            return Ok(None);
        }

        // We also like to return None for prev page when it's the first forward pagination.
        let mut opts = IndexMap::new();
        match self.options.page {
            Page::Unspecified => return Ok(None),
            Page::Seek(ref raw) if raw.is_empty() => return Ok(None),
            Page::Seek(_) | Page::SeekBackward(_) => {
                let seek = f(&self.records_and_total.first().unwrap().record);
                opts.insert("seek".into(), format!("-{}", encode_seek(seek)?));
            }
            Page::Numeric(_) => unreachable!(),
        };
        Ok(Some(opts))
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
    pub fn load<'a, U>(
        self,
        conn: &'a mut AsyncPgConnection,
    ) -> BoxFuture<'a, QueryResult<Paginated<U>>>
    where
        Self: diesel_async::methods::LoadQuery<'a, AsyncPgConnection, WithCount<U>>,
        T: 'a,
        U: Send + 'a,
    {
        use diesel_async::methods::LoadQuery;

        let options = self.options.clone();
        let future = self.internal_load(conn);

        async move {
            let mut records_and_total: Vec<_> = future.await?.try_collect().await?;

            // This maintain consistent ordering from page to pagen
            if options.is_backward() {
                records_and_total.reverse();
            }

            Ok(Paginated {
                records_and_total,
                options,
            })
        }
        .boxed()
    }
}

impl<T> QueryId for PaginatedQuery<T> {
    const HAS_STATIC_QUERY_ID: bool = false;
    type QueryId = ();
}

impl<T: Query> Query for PaginatedQuery<T> {
    type SqlType = (T::SqlType, BigInt);
}

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

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Function to check if the request is blocked.
///
/// A request can be blocked if either the User Agent is on the User Agent block list or if the client
/// IP is on the CIDR block list.
fn is_useragent_or_ip_blocked(config: &Server, req: &Parts) -> bool {
    let user_agent = req.headers.get_str_or_default(header::USER_AGENT);
    let client_ip = req.extensions.get::<RealIp>();

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

#[derive(Debug)]
pub(crate) struct PaginatedQueryWithCountSubq<T, C> {
    query: T,
    count_query: C,
    options: PaginationOptions,
}

impl<T, C> QueryId for PaginatedQueryWithCountSubq<T, C> {
    const HAS_STATIC_QUERY_ID: bool = false;
    type QueryId = ();
}

impl<T: Query, C: Query + QueryDsl + diesel::query_dsl::methods::SelectDsl<diesel::dsl::CountStar>>
    Query for PaginatedQueryWithCountSubq<T, C>
{
    type SqlType = (T::SqlType, BigInt);
}

impl<T, C> QueryFragment<Pg> for PaginatedQueryWithCountSubq<T, C>
where
    T: QueryFragment<Pg>,
    C: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_sql("SELECT *, (");
        self.count_query.walk_ast(out.reborrow())?;
        out.push_sql(") FROM (");
        self.query.walk_ast(out.reborrow())?;
        out.push_sql(") t LIMIT ");
        out.push_bind_param::<BigInt, _>(&self.options.per_page)?;
        if let Some(offset) = self.options.offset() {
            // Injection safety: `offset()` returns `Option<i64>`, so this interpolation is constrained to known
            // valid values and this is not vulnerable to user injection attacks.
            out.push_sql(format!(" OFFSET {offset}").as_str());
        }
        Ok(())
    }
}

impl<T, C> PaginatedQueryWithCountSubq<T, C> {
    pub fn load<'a, U>(
        self,
        conn: &'a mut AsyncPgConnection,
    ) -> BoxFuture<'a, QueryResult<Paginated<U>>>
    where
        Self: diesel_async::methods::LoadQuery<'a, AsyncPgConnection, WithCount<U>> + Send,
        C: 'a,
        T: 'a,
        U: Send + 'a,
    {
        use diesel_async::methods::LoadQuery;

        let options = self.options.clone();
        let future = self.internal_load(conn);

        async move {
            let mut records_and_total: Vec<_> = future.await?.try_collect().await?;

            // This maintain consistent ordering from page to pagen
            if options.is_backward() {
                records_and_total.reverse();
            }

            Ok(Paginated {
                records_and_total,
                options,
            })
        }
        .boxed()
    }
}

macro_rules! seek {
    // Field struct
    (@variant_struct $vis:vis $variant:ident {
        $($(#[$field_meta:meta])? $field:ident: $ty:ty),* $(,)?
    }) => {
        paste::item! {
            #[derive(Debug, Default, Deserialize, PartialEq)]
            #[serde(from = $variant "Helper")]
            $vis struct $variant {
                $($(#[$field_meta])? pub(super) $field: $ty),*
            }

            #[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
            struct [<$variant Helper>]($($(#[$field_meta])? pub(super) $ty),*);

            impl From<[<$variant Helper>]> for $variant {
                fn from(helper: [<$variant Helper>]) -> Self {
                    let [<$variant Helper>]($($field,)*) = helper;
                    Self { $($field,)* }
                }
            }

            impl serde::Serialize for $variant {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    let helper = [<$variant Helper>]($(self.$field,)*);
                    serde::Serialize::serialize(&helper, serializer)
                }
            }
        }
    };
    (
        $vis:vis enum $name:ident {
            $(
                $variant:ident $fields:tt,
            )*
        }
    ) => {
        $(
            seek!(@variant_struct $vis $variant $fields);
        )*
        paste::item! {
            #[derive(Debug, Deserialize, Serialize, PartialEq)]
            #[serde(untagged)]
            $vis enum [<$name Payload>] {
                $(
                    $variant($variant),
                )*
            }

            #[derive(Debug, PartialEq)]
            $vis enum $name {
                $(
                    $variant,
                )*
            }

            $(
                impl From<$variant> for [<$name Payload>] {
                    fn from(value: $variant) -> Self {
                        [<$name Payload>]::$variant(value)
                    }
                }
            )*
            impl From<[<$name Payload>]> for $name {
                fn from(value: [<$name Payload>]) -> Self {
                    match value {
                        $(
                            [<$name Payload>]::$variant(_) => $name::$variant,
                        )*
                    }
                }
            }

            use crate::util::errors::AppResult;
            use crate::controllers::helpers::pagination::Page;
            impl $name {
                pub fn decode(&self, page: &Page) -> AppResult<Option<[<$name Payload>]>> {
                    let encoded = match page {
                        Page::Seek(encoded) if !encoded.is_empty() => encoded,
                        Page::SeekBackward(encoded) if !encoded.is_empty() => encoded,
                        _ => return Ok(None),
                    };

                    Ok(Some(match self {
                        $(
                            $name::$variant => encoded.decode::<$variant>()?.into(),
                        )*
                    }))
                }
            }
        }
    };
}

pub(crate) use seek;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use http::{Method, Request, StatusCode};
    use insta::assert_snapshot;

    #[test]
    fn no_pagination_param() {
        let pagination = PaginationOptions::builder().gather(&mock("")).unwrap();
        assert_eq!(Page::Unspecified, pagination.page);
        assert_eq!(DEFAULT_PER_PAGE, pagination.per_page);
    }

    #[test]
    fn page_param_parsing() {
        let error = |query| pagination_error(PaginationOptions::builder(), query);

        assert_snapshot!(error("page="), @"Failed to deserialize query string: page: cannot parse integer from empty string");
        assert_snapshot!(error("page=not_a_number"), @"Failed to deserialize query string: page: invalid digit found in string");
        assert_snapshot!(error("page=1.0"), @"Failed to deserialize query string: page: invalid digit found in string");
        assert_snapshot!(error("page=0"), @"Failed to deserialize query string: page: invalid value: integer `0`, expected a nonzero u32");

        let pagination = PaginationOptions::builder()
            .gather(&mock("page=5"))
            .unwrap();
        assert_eq!(Page::Numeric(5), pagination.page);
    }

    #[test]
    fn per_page_param_parsing() {
        let error = |query| pagination_error(PaginationOptions::builder(), query);

        assert_snapshot!(error("per_page="), @"Failed to deserialize query string: per_page: cannot parse integer from empty string");
        assert_snapshot!(error("per_page=not_a_number"), @"Failed to deserialize query string: per_page: invalid digit found in string");
        assert_snapshot!(error("per_page=1.0"), @"Failed to deserialize query string: per_page: invalid digit found in string");
        assert_snapshot!(error("per_page=101"), @"cannot request more than 100 items");
        assert_snapshot!(error("per_page=0"), @"Failed to deserialize query string: per_page: invalid value: integer `0`, expected a nonzero u32");

        let pagination = PaginationOptions::builder()
            .gather(&mock("per_page=5"))
            .unwrap();
        assert_eq!(pagination.per_page, 5);
    }

    #[test]
    fn seek_param_parsing() {
        let error = pagination_error(PaginationOptions::builder(), "seek=OTg");
        assert_snapshot!(error, @"?seek= is not supported for this request");

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

        // for backward
        let error = pagination_error(PaginationOptions::builder(), "seek=-OTg");
        assert_snapshot!(error, @"seek backward ?seek=- is not supported for this request");

        let pagination = PaginationOptions::builder()
            .enable_seek_backward(true)
            .gather(&mock("seek=-OTg"))
            .unwrap();

        if let Page::SeekBackward(raw) = pagination.page {
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
        let error = pagination_error(PaginationOptions::builder(), "page=1&seek=OTg");
        assert_snapshot!(error, @"providing both ?page= and ?seek= is unsupported");

        let error = pagination_error(
            PaginationOptions::builder().enable_seek(true),
            "page=1&seek=OTg",
        );
        assert_snapshot!(error, @"providing both ?page= and ?seek= is unsupported");

        // for backward
        let error = pagination_error(
            PaginationOptions::builder().enable_seek_backward(true),
            "page=1&seek=-OTg",
        );
        assert_snapshot!(error, @"providing both ?page= and ?seek= is unsupported");
    }

    #[test]
    fn disabled_pages() {
        let error = pagination_error(PaginationOptions::builder().enable_pages(false), "page=1");
        assert_snapshot!(error, @"?page= is not supported for this request");
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

    mod seek {
        use chrono::Utc;
        use chrono::serde::ts_microseconds;

        seek!(
            pub(super) enum Seek {
                Id {
                    id: i32,
                },
                New {
                    #[serde(with = "ts_microseconds")]
                    dt: chrono::DateTime<Utc>,
                    id: i32,
                },
                RecentDownloads {
                    downloads: Option<i64>,
                    id: i32,
                },
            }
        );
    }

    #[test]
    fn test_seek_macro_encode_and_decode() {
        use chrono::NaiveDate;
        use chrono::serde::ts_microseconds;
        use seek::*;

        let assert_decode = |seek: Seek, payload: Option<_>| {
            for (param, prefix) in [("seek", ""), ("seek", "-")] {
                let query = if let Some(ref s) = payload {
                    &format!("{param}={prefix}{}", encode_seek(s).unwrap())
                } else {
                    ""
                };
                let pagination = PaginationOptions::builder()
                    .enable_seek(true)
                    .enable_seek_backward(true)
                    .gather(&mock(query))
                    .unwrap();
                let decoded = seek.decode(&pagination.page).unwrap();
                assert_eq!(decoded.as_ref(), payload.as_ref());
            }
        };

        let id = 1234;
        let seek = Seek::Id;
        let payload = SeekPayload::Id(Id { id });
        assert_decode(seek, Some(payload));

        let dt = NaiveDate::from_ymd_opt(2016, 7, 8)
            .unwrap()
            .and_hms_opt(9, 10, 11)
            .unwrap()
            .and_utc();
        let seek = Seek::New;
        let payload = SeekPayload::New(New { dt, id });
        assert_decode(seek, Some(payload));

        let downloads = Some(5678);
        let seek = Seek::RecentDownloads;
        let payload = SeekPayload::RecentDownloads(RecentDownloads { downloads, id });
        assert_decode(seek, Some(payload));

        let seek = Seek::Id;
        assert_decode(seek, None);

        // invalid seek payload
        {
            let seek = Seek::Id;
            let payload = SeekPayload::RecentDownloads(RecentDownloads { downloads, id });
            let query = format!("seek={}", encode_seek(&payload).unwrap());
            let pagination = PaginationOptions::builder()
                .enable_seek(true)
                .gather(&mock(&query))
                .unwrap();
            let error = seek.decode(&pagination.page).unwrap_err();
            assert_eq!(error.to_string(), "invalid seek parameter");
            let response = error.response();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);

            // for backward
            let query = format!("seek=-{}", encode_seek(&payload).unwrap());
            let pagination = PaginationOptions::builder()
                .enable_seek_backward(true)
                .gather(&mock(&query))
                .unwrap();
            let error = seek.decode(&pagination.page).unwrap_err();
            assert_eq!(error.to_string(), "invalid seek parameter");
            let response = error.response();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }

        // empty string
        {
            let seek = Seek::Id;
            let pagination = PaginationOptions::builder()
                .enable_seek(true)
                .gather(&mock("seek="))
                .unwrap();
            assert_eq!(seek.decode(&pagination.page).unwrap(), None);

            // for backward
            let seek = Seek::Id;
            let pagination = PaginationOptions::builder()
                .enable_seek_backward(true)
                .gather(&mock("seek=-"))
                .unwrap();
            assert_eq!(seek.decode(&pagination.page).unwrap(), None);
        }

        // Ensures it still encodes compactly with a field struct
        #[derive(Debug, Default, Serialize, PartialEq)]
        struct NewTuple(
            #[serde(with = "ts_microseconds")] chrono::DateTime<Utc>,
            i32,
        );
        assert_eq!(
            encode_seek(NewTuple(dt, id)).unwrap(),
            encode_seek(SeekPayload::New(New { dt, id })).unwrap()
        );
    }

    #[test]
    fn test_seek_macro_conv() {
        use chrono::NaiveDate;
        use seek::*;
        let id = 1234;
        assert_eq!(Seek::from(SeekPayload::Id(Id { id })), Seek::Id);

        let dt = NaiveDate::from_ymd_opt(2016, 7, 8)
            .unwrap()
            .and_hms_opt(9, 10, 11)
            .unwrap()
            .and_utc();
        assert_eq!(Seek::from(SeekPayload::New(New { dt, id })), Seek::New);

        let downloads = None;
        assert_eq!(
            Seek::from(SeekPayload::RecentDownloads(RecentDownloads {
                downloads,
                id
            })),
            Seek::RecentDownloads
        );
    }

    fn mock(query: &str) -> Parts {
        Request::builder()
            .method(Method::GET)
            .uri(format!("/?{query}"))
            .body(())
            .unwrap()
            .into_parts()
            .0
    }

    fn pagination_error(options: PaginationOptionsBuilder, query: &str) -> String {
        let error = options.gather(&mock(query)).unwrap_err();

        let response = error.response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        error.to_string()
    }
}
