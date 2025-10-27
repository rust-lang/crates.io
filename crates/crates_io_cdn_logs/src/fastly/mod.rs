//! # Fastly CDN log parsing
//!
//! see <https://docs.fastly.com/en/guides/changing-log-line-formats#classic-format>.

mod json;

use crate::DownloadsMap;
use crate::paths::parse_path;
use std::borrow::Cow;
use tokio::io::{AsyncBufRead, AsyncBufReadExt};
use tracing::{debug_span, instrument, warn};

#[instrument(level = "debug", skip(reader))]
pub async fn count_downloads(reader: impl AsyncBufRead + Unpin) -> anyhow::Result<DownloadsMap> {
    let mut downloads = DownloadsMap::new();

    let mut lines = reader.lines();
    while let Some(line) = lines.next_line().await? {
        let span = debug_span!("process_line");
        let _guard = span.enter();

        let Some(json) = parse_line(&line) else {
            warn!("Failed to find JSON start");
            continue;
        };

        let json = match parse_json(json) {
            Ok(json) => json,
            Err(error) => {
                warn!("Failed to parse JSON: {error}");
                continue;
            }
        };

        if json.method() != "GET" {
            // Ignore non-GET requests.
            continue;
        }

        if json.status() != 200 {
            // Ignore non-200 responses.
            continue;
        }

        let url = decode_url(json.url());

        // We're avoiding parsing to `url::Url` here for performance reasons.
        // Since we're already filtering out non-200 responses, we can assume
        // that the URL is valid.

        let Some((name, version)) = parse_path(&url) else {
            continue;
        };

        let date = json.date_time().date_naive();

        downloads.add(name, version, date);
    }

    Ok(downloads)
}

#[instrument(level = "debug", skip(line))]
fn parse_line(line: &str) -> Option<&str> {
    // A regex could also be used here, but the `find()` call appears to
    // be roughly 10x faster.
    line.find(r#"]: {"#).map(|pos| &line[pos + 3..])
}

#[instrument(level = "debug", skip(json))]
fn parse_json(json: &str) -> Result<json::LogLine<'_>, serde_json::Error> {
    serde_json::from_str(json)
}

/// Deal with paths like `/crates/tikv-jemalloc-sys/tikv-jemalloc-sys-0.5.4%2B5.3.0-patched.crate`.
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
    use insta::assert_debug_snapshot;
    use std::io::Cursor;

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
            2025-10-26  matrixmultiply@0.3.3 .. 1
            2025-10-26  owo-colors@4.2.3 .. 1
            2025-10-26  parking_lot@0.12.5 .. 1
            2025-10-26  precis-profiles@0.1.11 .. 1
            2025-10-26  precis-tools@0.1.8 .. 1
            2025-10-26  rand@0.8.5 .. 1
            2025-10-26  scale-info@2.11.3 .. 1
            2025-10-26  tinyvec_macros@0.1.1 .. 1
            2025-10-26  tower@0.5.2 .. 1
            2025-10-26  unicode-normalization@0.1.22 .. 1
        }
        ");
    }
}
