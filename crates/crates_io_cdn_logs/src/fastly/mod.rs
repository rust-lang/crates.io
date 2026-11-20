//! # Fastly CDN log parsing
//!
//! see <https://docs.fastly.com/en/guides/changing-log-line-formats#classic-format>.

mod json;

use crate::DownloadsMap;
use crate::paths::parse_path;
use crate::user_agent::should_count_user_agent;
use std::borrow::Cow;
use tokio::io::{AsyncBufRead, AsyncBufReadExt};
use tracing::{debug_span, instrument, warn};

#[instrument(level = "debug", skip(reader))]
pub async fn count_downloads(
    mut reader: impl AsyncBufRead + Unpin,
) -> anyhow::Result<DownloadsMap> {
    let mut downloads = DownloadsMap::new();

    let mut line = Vec::new();
    while reader.read_until(b'\n', &mut line).await? > 0 {
        let trimmed = line.strip_suffix(b"\n").unwrap_or(&line);
        let trimmed = trimmed.strip_suffix(b"\r").unwrap_or(trimmed);
        process_line(str::from_utf8(trimmed)?, &mut downloads);
        line.clear();
    }

    Ok(downloads)
}

fn process_line(line: &str, downloads: &mut DownloadsMap) {
    let span = debug_span!("process_line");
    let _guard = span.enter();

    let Some(json) = parse_line(line) else {
        warn!("Failed to find JSON start");
        return;
    };

    let json = match parse_json(json) {
        Ok(json) => json,
        Err(error) => {
            warn!("Failed to parse JSON: {error}");
            return;
        }
    };

    if json.version() != "1" {
        warn!("Unsupported log line version: {}", json.version());
        return;
    }

    if json.method() != "GET" {
        // Ignore non-GET requests.
        return;
    }

    if json.status() != 200 {
        // Ignore non-200 responses.
        return;
    }

    if json
        .user_agent()
        .is_some_and(|ua| !should_count_user_agent(ua))
    {
        // Ignore requests from user agents that should not be counted.
        return;
    }

    let url = decode_url(json.url());

    // We're avoiding parsing to `url::Url` here for performance reasons.
    // Since we're already filtering out non-200 responses, we can assume
    // that the URL is valid.

    let Some((name, version)) = parse_path(&url) else {
        return;
    };

    let Some(date) = json.date() else {
        warn!("Failed to parse date `{}`", json.date_time);
        return;
    };

    downloads.add(name, version, date);
}

/// Strips the optional `classic` syslog prefix (`<134>... s3-request-logs[322614]: `).
#[instrument(level = "debug", skip(line))]
fn parse_line(line: &str) -> Option<&str> {
    match line.strip_prefix('<') {
        Some(rest) => rest.split_once("]: ").map(|(_, json)| json),
        None => Some(line),
    }
}

#[instrument(level = "debug", skip(json))]
fn parse_json(json: &str) -> Result<json::LogLine<'_>, serde_json::Error> {
    serde_json::from_str(json)
}

/// Deals with paths like `/crates/tikv-jemalloc-sys/tikv-jemalloc-sys-0.5.4%2B5.3.0-patched.crate`.
///
/// Compared to the CloudFront logs, we only need a single round of
/// percent-decoding here, since JSON has its own escaping rules.
#[instrument(level = "debug", skip(url))]
fn decode_url(url: &str) -> Cow<'_, str> {
    percent_encoding::percent_decode_str(url).decode_utf8_lossy()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use claims::assert_ok;
    use insta::{assert_debug_snapshot, assert_snapshot};
    use std::io::Cursor;
    use tokio::io::BufReader;

    const LINE: &str = r#"<134>2024-01-16T23:53:20Z cache-iad-kiad7000128 s3-request-logs[322614]: {"bytes":11355,"date_time":"2024-01-16T23:53:20.460557177Z","ip":"1.2.3.4","method":"GET","status":200,"url":"https://static.crates.io/crates/strsim/strsim-0.10.0.crate","version":"1"}"#;

    #[tokio::test]
    async fn test_basic() {
        let _guard = enable_tracing_output();

        let mut cursor = Cursor::new(include_bytes!("../../test_data/fastly/basic.log"));
        let downloads = assert_ok!(count_downloads(&mut cursor).await);

        assert_debug_snapshot!(downloads, @r"
        DownloadsMap {
            2024-01-16  strsim@0.10.0 .. 1
            2024-01-16  tikv-jemalloc-sys@0.5.2+5.3.0-patched .. 1
            2024-01-16  tinyvec@1.6.0 .. 1
            2024-01-16  winapi-x86_64-pc-windows-gnu@0.4.0 .. 1
            2024-01-16  windows_x86_64_gnu@0.48.0 .. 1
            2024-01-16  windows_x86_64_gnullvm@0.42.2 .. 1
            2024-01-16  winnow@0.5.4 .. 1
            2024-01-17  anstyle@1.0.1 .. 1
            2024-01-17  cast@0.3.0 .. 1
            2024-01-17  cc@1.0.73 .. 1
            2024-01-17  croaring-sys@1.1.0 .. 1
            2024-01-17  half@1.8.2 .. 1
            2024-01-17  jemalloc-sys@0.3.2 .. 1
            2024-01-17  lazy_static@1.4.0 .. 1
            2024-01-17  libc@0.2.126 .. 1
            2024-01-17  lzma-sys@0.1.20 .. 1
            2024-01-17  sqlparser@0.40.0 .. 1
            2024-01-17  synchronized-writer@1.1.11 .. 1
            2024-01-17  tikv-jemalloc-sys@0.5.4+5.3.0-patched .. 1
            2024-01-17  windows_x86_64_gnu@0.48.0 .. 2
            2024-01-17  xz2@0.1.7 .. 1
            2024-01-17  zstd-safe@7.0.0 .. 1
        }
        ");
    }

    #[tokio::test]
    async fn test_percent_encoding() {
        let _guard = enable_tracing_output();

        let mut cursor = Cursor::new(include_bytes!(
            "../../test_data/fastly/percent-encoding.log"
        ));
        let downloads = assert_ok!(count_downloads(&mut cursor).await);

        assert_debug_snapshot!(downloads, @r"
        DownloadsMap {
            2024-01-16  tikv-jemalloc-sys@0.5.2+5.3.0-patched .. 2
        }
        ");
    }

    #[tokio::test]
    async fn test_unrelated_traffic() {
        let _guard = enable_tracing_output();

        let mut cursor = Cursor::new(include_bytes!(
            "../../test_data/fastly/unrelated-traffic.log"
        ));
        let downloads = assert_ok!(count_downloads(&mut cursor).await);

        assert_debug_snapshot!(downloads, @r"
        DownloadsMap {
            2024-01-16  strsim@0.10.0 .. 2
        }
        ");
    }

    #[tokio::test]
    async fn test_recoverable_errors() {
        let _guard = enable_tracing_output();

        let mut cursor = Cursor::new(include_bytes!(
            "../../test_data/fastly/recoverable-errors.log"
        ));
        let downloads = assert_ok!(count_downloads(&mut cursor).await);

        assert_debug_snapshot!(downloads, @r"
        DownloadsMap {
            2024-01-16  strsim@0.10.0 .. 1
        }
        ");
    }

    #[tokio::test]
    async fn test_full_info() {
        let _guard = enable_tracing_output();

        let mut cursor = Cursor::new(include_bytes!("../../test_data/fastly/full-info.log"));
        let downloads = assert_ok!(count_downloads(&mut cursor).await);

        assert_debug_snapshot!(downloads, @r"
        DownloadsMap {
            2025-10-26  cargo-set-version@0.0.2 .. 1
            2025-10-26  dashmap@6.1.0 .. 1
            2025-10-26  gix-packetline@0.19.3 .. 1
            2025-10-26  gix-refspec@0.30.1 .. 1
            2025-10-26  http@1.3.1 .. 1
            2025-10-26  http-body@1.0.1 .. 1
            2025-10-26  indexmap@2.12.0 .. 1
            2025-10-26  ipnet@2.11.0 .. 1
            2025-10-26  libc@0.2.177 .. 1
            2025-10-26  lru-slab@0.1.2 .. 1
            2025-10-26  owo-colors@4.2.3 .. 1
            2025-10-26  parking_lot@0.12.5 .. 1
            2025-10-26  precis-profiles@0.1.11 .. 1
            2025-10-26  precis-tools@0.1.8 .. 1
            2025-10-26  scale-info@2.11.3 .. 1
            2025-10-26  tinyvec_macros@0.1.1 .. 1
            2025-10-26  unicode-normalization@0.1.22 .. 1
        }
        ");
    }

    #[tokio::test]
    async fn test_small_read_buffer() {
        let _guard = enable_tracing_output();

        let bytes = include_bytes!("../../test_data/fastly/basic.log");
        let expected = assert_ok!(count_downloads(&bytes[..]).await);

        let reader = BufReader::with_capacity(7, &bytes[..]);
        let downloads = assert_ok!(count_downloads(reader).await);
        assert_eq!(*downloads, *expected);
    }

    #[tokio::test]
    async fn test_line_endings() {
        let _guard = enable_tracing_output();

        let input = format!("{LINE}\n\n{LINE}\r\n{LINE}");
        let downloads = assert_ok!(count_downloads(input.as_bytes()).await);

        assert_debug_snapshot!(downloads, @"
        DownloadsMap {
            2024-01-16  strsim@0.10.0 .. 3
        }
        ");
    }

    #[tokio::test]
    async fn test_invalid_utf8() {
        let _guard = enable_tracing_output();

        let input = [LINE.as_bytes(), b"\n<134>\xff\n", LINE.as_bytes()].concat();
        let error = count_downloads(&input[..]).await.unwrap_err();

        assert_snapshot!(error, @"invalid utf-8 sequence of 1 bytes from index 5");
    }

    #[tokio::test]
    async fn test_without_prefix() {
        let _guard = enable_tracing_output();

        let (_, json) = LINE.split_once("]: ").unwrap();
        let input = format!("{json}\n{LINE}\n");
        let downloads = assert_ok!(count_downloads(input.as_bytes()).await);

        assert_debug_snapshot!(downloads, @r"
        DownloadsMap {
            2024-01-16  strsim@0.10.0 .. 2
        }
        ");
    }
}
