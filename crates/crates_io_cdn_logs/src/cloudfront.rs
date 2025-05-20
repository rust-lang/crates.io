//! # CloudFront log parsing
//!
//! see <https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/AccessLogs.html#LogFileFormat>
//! and <https://www.w3.org/TR/WD-logfile.html>.

use crate::DownloadsMap;
use crate::paths::parse_path;
use chrono::NaiveDate;
use std::borrow::Cow;
use tokio::io::{AsyncBufRead, AsyncBufReadExt};
use tracing::{instrument, warn};

const HEADER_PREFIX: char = '#';
const HEADER_VERSION: &str = "#Version:";
const HEADER_FIELDS: &str = "#Fields:";

const FIELD_DATE: &str = "date";
const FIELD_METHOD: &str = "cs-method";
const FIELD_PATH: &str = "cs-uri-stem";
const FIELD_STATUS: &str = "sc-status";

#[instrument(level = "debug", skip(reader))]
pub async fn count_downloads(reader: impl AsyncBufRead + Unpin) -> anyhow::Result<DownloadsMap> {
    let mut num_fields = 0;
    let mut date_index = None;
    let mut method_index = None;
    let mut path_index = None;
    let mut status_index = None;

    let mut downloads = DownloadsMap::new();

    let mut lines = reader.lines();
    while let Some(line) = lines.next_line().await? {
        if let Some(version) = line.strip_prefix(HEADER_VERSION) {
            let version = version.trim();
            if version != "1.0" {
                anyhow::bail!("Unsupported version: {}", version);
            }
            continue;
        }

        if let Some(fields_str) = line.strip_prefix(HEADER_FIELDS) {
            let fields = fields_str.trim().split(' ').collect::<Vec<_>>();

            num_fields = fields.len();
            date_index = fields.iter().position(|f| f == &FIELD_DATE);
            method_index = fields.iter().position(|f| f == &FIELD_METHOD);
            path_index = fields.iter().position(|f| f == &FIELD_PATH);
            status_index = fields.iter().position(|f| f == &FIELD_STATUS);

            continue;
        }

        if line.starts_with(HEADER_PREFIX) {
            warn!("Unexpected log header line: {}", line);
            continue;
        }

        let values = line.split('\t').collect::<Vec<_>>();

        let num_values = values.len();
        if num_values != num_fields {
            warn!("Expected {num_fields} fields, but found {num_values}");
            continue;
        }

        let method = get_value(&values, method_index, FIELD_METHOD);
        if method != "GET" {
            // Ignore non-GET requests.
            continue;
        }

        let status = get_value(&values, status_index, FIELD_STATUS);
        if status != "200" {
            // Ignore non-200 responses.
            continue;
        }

        let path = get_value(&values, path_index, FIELD_PATH);

        // Deal with paths like `/crates/tikv-jemalloc-sys/tikv-jemalloc-sys-0.5.4%252B5.3.0-patched.crate`.
        //
        // Yes, the second round of decoding is intentional, since cargo is
        // requesting crates with a percent-encoded path, and then CloudFront
        // is percent-encoding that percent-encoded path again when logging it.
        let path = decode_path(path);
        let path = decode_path(&path);

        let Some((name, version)) = parse_path(&path) else {
            continue;
        };

        let date = get_value(&values, date_index, FIELD_DATE);
        let date = match date.parse::<NaiveDate>() {
            Ok(date) => date,
            Err(error) => {
                warn!(%date, %error, "Failed to parse date");
                continue;
            }
        };

        downloads.add(name, version, date);
    }

    Ok(downloads)
}

#[instrument(level = "debug", skip(path))]
fn decode_path(path: &str) -> Cow<'_, str> {
    percent_encoding::percent_decode_str(path).decode_utf8_lossy()
}

fn get_value<'a>(values: &'a [&'a str], index: Option<usize>, field_name: &'static str) -> &'a str {
    index
        .and_then(|i| values.get(i))
        .copied()
        .unwrap_or_else(|| {
            warn!(?index, "Failed to find {field_name} field.");
            ""
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use claims::{assert_err, assert_ok};
    use insta::{assert_debug_snapshot, assert_snapshot};
    use std::io::Cursor;

    #[tokio::test]
    async fn test_basic() {
        let _guard = enable_tracing_output();

        let mut cursor = Cursor::new(include_bytes!("../test_data/cloudfront/basic.log"));
        let downloads = assert_ok!(count_downloads(&mut cursor).await);

        assert_debug_snapshot!(downloads, @r"
        DownloadsMap {
            2024-01-16  bindgen@0.65.1 .. 1
            2024-01-16  cumulus-primitives-core@0.4.0 .. 1
            2024-01-16  derive_more@0.99.17 .. 1
            2024-01-16  hash-db@0.15.2 .. 1
            2024-01-16  hyper-rustls@0.24.2 .. 1
            2024-01-16  jsonrpsee-server@0.16.3 .. 1
            2024-01-16  peeking_take_while@0.1.2 .. 1
            2024-01-16  quick-error@1.2.3 .. 2
            2024-01-16  tracing-core@0.1.32 .. 1
            2024-01-17  flatbuffers@23.1.21 .. 1
            2024-01-17  jemallocator@0.5.4 .. 1
            2024-01-17  leveldb-sys@2.0.9 .. 1
            2024-01-17  num_cpus@1.15.0 .. 1
            2024-01-17  paste@1.0.12 .. 1
            2024-01-17  quick-error@1.2.3 .. 1
            2024-01-17  rand@0.8.5 .. 1
            2024-01-17  serde_derive@1.0.163 .. 1
            2024-01-17  smallvec@1.10.0 .. 1
            2024-01-17  tar@0.4.38 .. 1
        }
        ");
    }

    #[tokio::test]
    async fn test_percent_encoding() {
        let _guard = enable_tracing_output();

        let mut cursor = Cursor::new(include_bytes!(
            "../test_data/cloudfront/percent-encoding.log"
        ));
        let downloads = assert_ok!(count_downloads(&mut cursor).await);

        assert_debug_snapshot!(downloads, @r"
        DownloadsMap {
            2024-01-17  zstd-sys@2.0.8+zstd.1.5.5 .. 3
        }
        ");
    }

    #[tokio::test]
    async fn test_unrelated_traffic() {
        let _guard = enable_tracing_output();

        let mut cursor = Cursor::new(include_bytes!(
            "../test_data/cloudfront/unrelated-traffic.log"
        ));
        let downloads = assert_ok!(count_downloads(&mut cursor).await);

        assert_debug_snapshot!(downloads, @r"
        DownloadsMap {
            2024-01-16  bindgen@0.65.1 .. 2
        }
        ");
    }

    #[tokio::test]
    async fn test_recoverable_errors() {
        let _guard = enable_tracing_output();

        let mut cursor = Cursor::new(include_bytes!(
            "../test_data/cloudfront/recoverable-errors.log"
        ));
        let downloads = assert_ok!(count_downloads(&mut cursor).await);

        assert_debug_snapshot!(downloads, @r"
        DownloadsMap {
            2024-01-16  bindgen@0.65.1 .. 1
        }
        ");
    }

    #[tokio::test]
    async fn test_unknown_version() {
        let _guard = enable_tracing_output();

        let mut cursor = Cursor::new(include_bytes!(
            "../test_data/cloudfront/unknown-version.log"
        ));
        let error = assert_err!(count_downloads(&mut cursor).await);

        assert_snapshot!(error, @"Unsupported version: 2.0");
    }
}
