pub mod cloudfront;
mod compression;
mod download_map;
pub mod fastly;
mod paths;
#[cfg(test)]
mod test_utils;

pub use crate::compression::Decompressor;
pub use crate::download_map::DownloadsMap;
use std::io::Cursor;
use tokio::io::{AsyncBufRead, AsyncReadExt};
use tracing::instrument;

#[instrument(skip_all)]
pub async fn count_downloads<R>(mut reader: R) -> anyhow::Result<DownloadsMap>
where
    R: AsyncBufRead + Unpin,
{
    // Read the first byte to determine the file format.
    match reader.read_u8().await? {
        // CloudFront log files start with a `#Version` header.
        b'#' => {
            // We can't use `AsyncSeek` here because `async-compression` does
            // not support it, but we can use `Cursor` to prepend the `#` back
            // onto the reader.
            let reader = Cursor::new(b"#").chain(reader);
            cloudfront::count_downloads(reader).await
        }
        // Fastly log lines start with a `<123>` field.
        b'<' => {
            // We can't use `AsyncSeek` here because `async-compression` does
            // not support it, but we can use `Cursor` to prepend the `<` back
            // onto the reader.
            let reader = Cursor::new(b"<").chain(reader);
            fastly::count_downloads(reader).await
        }
        // Anything else is rejected.
        byte => {
            anyhow::bail!("Failed to determine log file format. Unrecognized first byte: {byte:?}.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compression::Decompressor;
    use crate::test_utils::*;
    use claims::{assert_err, assert_ok};
    use insta::{assert_debug_snapshot, assert_snapshot};
    use std::io::Cursor;

    #[tokio::test]
    async fn test_cloudfront() {
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
    async fn test_compressed_cloudfront() {
        let _guard = enable_tracing_output();

        let cursor = Cursor::new(include_bytes!("../test_data/cloudfront/basic.log.gz"));

        let decompressor = assert_ok!(Decompressor::from_extension(cursor, Some("gz")));
        let reader = tokio::io::BufReader::new(decompressor);

        let downloads = assert_ok!(count_downloads(reader).await);

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
    async fn test_fastly() {
        let _guard = enable_tracing_output();

        let mut cursor = Cursor::new(include_bytes!("../test_data/fastly/basic.log"));
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
    async fn test_compressed_fastly() {
        let _guard = enable_tracing_output();

        let cursor = Cursor::new(include_bytes!("../test_data/fastly/basic.log.zst"));

        let decompressor = assert_ok!(Decompressor::from_extension(cursor, Some("zst")));
        let reader = tokio::io::BufReader::new(decompressor);

        let downloads = assert_ok!(count_downloads(reader).await);

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
    async fn test_unknown() {
        let _guard = enable_tracing_output();

        let mut cursor = Cursor::new(b"foo");
        let error = assert_err!(count_downloads(&mut cursor).await);
        assert_snapshot!(error, @"Failed to determine log file format. Unrecognized first byte: 102.");
    }
}
