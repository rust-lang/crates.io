use std::io;
use std::io::prelude::*;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};

#[derive(Debug)]
pub struct LimitErrorReader<R> {
    inner: io::Take<R>,
}

impl<R: Read> LimitErrorReader<R> {
    pub fn new(r: R, limit: u64) -> LimitErrorReader<R> {
        LimitErrorReader {
            inner: r.take(limit),
        }
    }
}

impl<R: Read> Read for LimitErrorReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.inner.read(buf) {
            Ok(0) if self.inner.limit() == 0 => Err(io::Error::new(
                io::ErrorKind::Other,
                "maximum limit reached when reading",
            )),
            e => e,
        }
    }
}

#[derive(Debug)]
pub struct AsyncLimitErrorReader<R> {
    inner: tokio::io::Take<R>,
}

impl<R: AsyncRead + Unpin> AsyncLimitErrorReader<R> {
    pub fn new(r: R, limit: u64) -> AsyncLimitErrorReader<R> {
        AsyncLimitErrorReader {
            inner: r.take(limit),
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for AsyncLimitErrorReader<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let reader = Pin::new(&mut self.inner);
        match reader.poll_read(cx, buf) {
            Poll::Ready(Ok(())) if self.inner.limit() == 0 => Poll::Ready(Err(io::Error::new(
                io::ErrorKind::Other,
                "maximum limit reached when reading",
            ))),
            e => e,
        }
    }
}
