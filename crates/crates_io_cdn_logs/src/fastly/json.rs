//! Imported from <https://github.com/rust-lang/simpleinfra/blob/4fb365809295de075d28d8b2d51f6f419537be7d/terragrunt/modules/crates-io/compute-static/src/log_line.rs>

use crate::date::parse_date;
use chrono::NaiveDate;
use serde::Deserialize;
use std::borrow::Cow;

/// This struct corresponds to the JSON payload of a log line from
/// Fastly's CDN logs.
///
/// Compared to the implementation in the [rust-lang/simpleinfra](https://github.com/rust-lang/simpleinfra/)
/// repository, there are a couple of differences:
///
/// - The `bytes` field is not included, because we don't need it.
/// - The `ip` field is not included, because we don't need it.
/// - The `method` and `status` fields are not optional, because we handle
///   parsing errors gracefully.
/// - The `date_time` field is kept as a borrowed string and only its date
///   portion is parsed on demand (see [`LogLine::date`]), to avoid parsing the
///   full timestamp for every line when we only need the date.
/// - The `date_time`, `method`, `url`, and `version` fields are using `Cow` to
///   avoid unnecessary allocations.
///
/// The `version` field is deserialized as a plain struct field rather than a
/// serde tag, because an internally tagged enum forces serde to buffer the
/// whole payload into an intermediate representation before dispatching.
#[derive(Debug, Deserialize)]
pub struct LogLine<'a> {
    #[serde(borrow)]
    pub version: Cow<'a, str>,
    #[serde(borrow)]
    pub date_time: Cow<'a, str>,
    #[serde(borrow)]
    pub method: Cow<'a, str>,
    #[serde(borrow)]
    pub url: Cow<'a, str>,
    pub status: u16,
    #[serde(borrow)]
    pub http: Option<Http<'a>>,
}

impl LogLine<'_> {
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Parses the date portion (`YYYY-MM-DD`) from the start of the `date_time`
    /// timestamp. Fastly emits UTC timestamps, so this is the UTC date.
    pub fn date(&self) -> Option<NaiveDate> {
        self.date_time.get(..10).and_then(parse_date)
    }

    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn user_agent(&self) -> Option<&str> {
        self.http
            .as_ref()
            .and_then(|http| http.useragent.as_deref())
    }
}

#[derive(Debug, Deserialize)]
pub struct Http<'a> {
    #[serde(borrow)]
    pub useragent: Option<Cow<'a, str>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_ok;
    use insta::assert_debug_snapshot;

    #[test]
    fn test_parse() {
        let input = r#"{"bytes":null,"date_time":"2024-01-16T16:03:04.44007323Z","ip":"45.79.107.220","method":"GET","status":403,"url":"https://static.staging.crates.io/?1705420437","version":"1"}"#;
        let output = assert_ok!(serde_json::from_str::<LogLine<'_>>(input));
        assert_debug_snapshot!(output, @r#"
        LogLine {
            version: "1",
            date_time: "2024-01-16T16:03:04.44007323Z",
            method: "GET",
            url: "https://static.staging.crates.io/?1705420437",
            status: 403,
            http: None,
        }
        "#);

        assert_eq!(output.date().unwrap().to_string(), "2024-01-16");
        assert_eq!(output.method(), "GET");
        assert_eq!(output.url(), "https://static.staging.crates.io/?1705420437");
        assert_eq!(output.status(), 403);
        assert_eq!(output.user_agent(), None);

        assert!(is_borrowed(&output.method));
        assert!(is_borrowed(&output.url));
    }

    #[test]
    fn test_parse_with_user_agent() {
        let input = r#"{"bytes":36308,"content_type":"application/gzip","date_time":"2025-10-26T23:57:34.867635728Z","http":{"protocol":"HTTP/2","referer":null,"useragent":"cargo/1.92.0-nightly (344c4567c 2025-10-21)"},"ip":"192.0.2.1","method":"GET","status":200,"url":"https://static.crates.io/crates/scale-info/2.11.3/download","version":"1"}"#;
        let output = assert_ok!(serde_json::from_str::<LogLine<'_>>(input));
        assert_debug_snapshot!(output, @r#"
        LogLine {
            version: "1",
            date_time: "2025-10-26T23:57:34.867635728Z",
            method: "GET",
            url: "https://static.crates.io/crates/scale-info/2.11.3/download",
            status: 200,
            http: Some(
                Http {
                    useragent: Some(
                        "cargo/1.92.0-nightly (344c4567c 2025-10-21)",
                    ),
                },
            ),
        }
        "#);

        assert_eq!(output.date().unwrap().to_string(), "2025-10-26");
        assert_eq!(output.method(), "GET");
        assert_eq!(
            output.url(),
            "https://static.crates.io/crates/scale-info/2.11.3/download"
        );
        assert_eq!(output.status(), 200);
        assert_eq!(
            output.user_agent(),
            Some("cargo/1.92.0-nightly (344c4567c 2025-10-21)")
        );
    }

    #[allow(clippy::ptr_arg)]
    fn is_borrowed(s: &Cow<'_, str>) -> bool {
        match s {
            Cow::Borrowed(_) => true,
            Cow::Owned(_) => false,
        }
    }
}
