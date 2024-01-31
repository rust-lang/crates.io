use async_compression::tokio::bufread::{GzipDecoder, ZstdDecoder};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncBufRead, AsyncRead, ReadBuf};

/// A wrapper for the compression formats that CDN logs are currently stored in.
pub enum Decompressor<T> {
    Gzip(GzipDecoder<T>),
    Zstd(ZstdDecoder<T>),
}

impl<T: AsyncBufRead> Decompressor<T> {
    pub fn from_extension(inner: T, extension: Option<&str>) -> anyhow::Result<Self> {
        match extension {
            Some("gz") => Ok(Decompressor::gzip(inner)),
            Some("zst") => Ok(Decompressor::zstd(inner)),
            Some(ext) => anyhow::bail!("Unexpected file extension: {}", ext),
            None => anyhow::bail!("Unexpected missing file extension"),
        }
    }

    pub fn gzip(inner: T) -> Self {
        Decompressor::Gzip(GzipDecoder::new(inner))
    }

    pub fn zstd(inner: T) -> Self {
        Decompressor::Zstd(ZstdDecoder::new(inner))
    }
}

impl<T: AsyncBufRead + Unpin> AsyncRead for Decompressor<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match &mut *self {
            Decompressor::Gzip(inner) => Pin::new(inner).poll_read(cx, buf),
            Decompressor::Zstd(inner) => Pin::new(inner).poll_read(cx, buf),
        }
    }
}
