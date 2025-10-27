//! Imported from <https://github.com/rust-lang/simpleinfra/blob/4fb365809295de075d28d8b2d51f6f419537be7d/terragrunt/modules/crates-io/compute-static/src/log_line.rs>

use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::borrow::Cow;

/// This struct corresponds to the JSON payload of a log line from
/// Fastly's CDN logs.
#[derive(Debug, Deserialize)]
#[serde(tag = "version")]
pub enum LogLine<'a> {
    #[serde(borrow, rename = "1")]
    V1(LogLineV1<'a>),
}

impl LogLine<'_> {
    pub fn date_time(&self) -> DateTime<Utc> {
        match self {
            LogLine::V1(line) => line.date_time,
        }
    }

    pub fn method(&self) -> &str {
        match self {
            LogLine::V1(line) => &line.method,
        }
    }

    pub fn url(&self) -> &str {
        match self {
            LogLine::V1(line) => &line.url,
        }
    }

    pub fn status(&self) -> u16 {
        match self {
            LogLine::V1(line) => line.status,
        }
    }

    pub fn user_agent(&self) -> Option<&str> {
        match self {
            LogLine::V1(line) => line
                .http
                .as_ref()
                .and_then(|http| http.useragent.as_deref()),
        }
    }
}

/// This struct corresponds to the `"version": "1"` variant of the [LogLine] enum.
///
/// Compared to the implementation in the [rust-lang/simpleinfra](https://github.com/rust-lang/simpleinfra/)
/// repository, there are a couple of differences:
///
/// - The `bytes` field is not included, because we don't need it.
/// - The `ip` field is not included, because we don't need it.
/// - The `method` and `status` fields are not optional, because we handle
///   parsing errors gracefully.
/// - The `date_time` field is using `chrono` like the rest of the
///   crates.io codebase.
/// - The `method` and `url` fields are using `Cow` to avoid
///   unnecessary allocations.
#[derive(Debug, Deserialize)]
pub struct LogLineV1<'a> {
    pub date_time: DateTime<Utc>,
    #[serde(borrow)]
    pub method: Cow<'a, str>,
    #[serde(borrow)]
    pub url: Cow<'a, str>,
    pub status: u16,
    #[serde(borrow)]
    pub http: Option<Http<'a>>,
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
        V1(
            LogLineV1 {
                date_time: 2024-01-16T16:03:04.440073230Z,
                method: "GET",
                url: "https://static.staging.crates.io/?1705420437",
                status: 403,
                http: None,
            },
        )
        "#);

        assert_eq!(
            output.date_time().to_string(),
            "2024-01-16 16:03:04.440073230 UTC"
        );
        assert_eq!(output.method(), "GET");
        assert_eq!(output.url(), "https://static.staging.crates.io/?1705420437");
        assert_eq!(output.status(), 403);
        assert_eq!(output.user_agent(), None);

        match output {
            LogLine::V1(l) => {
                assert!(is_borrowed(&l.method));
                assert!(is_borrowed(&l.url));
            }
        }
    }

    #[test]
    fn test_parse_with_user_agent() {
        let input = r#"{"bytes":36308,"content_type":"application/gzip","date_time":"2025-10-26T23:57:34.867635728Z","http":{"protocol":"HTTP/2","referer":null,"useragent":"cargo/1.92.0-nightly (344c4567c 2025-10-21)"},"ip":"192.0.2.1","method":"GET","status":200,"url":"https://static.crates.io/crates/scale-info/2.11.3/download","version":"1"}"#;
        let output = assert_ok!(serde_json::from_str::<LogLine<'_>>(input));
        assert_debug_snapshot!(output, @r#"
        V1(
            LogLineV1 {
                date_time: 2025-10-26T23:57:34.867635728Z,
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
            },
        )
        "#);

        assert_eq!(
            output.date_time().to_string(),
            "2025-10-26 23:57:34.867635728 UTC"
        );
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
