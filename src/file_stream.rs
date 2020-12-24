use std::task::{Context, Poll};
use std::{io::Error, pin::Pin};

use bytes::Bytes;
use hyper::body::Body;
use tokio::{fs::File, io::AsyncRead};
use tokio_stream::Stream;

const BUFFER_SIZE: usize = 8 * 1024;

pub struct FileStream {
    file: File,
    buffer: Box<[u8; BUFFER_SIZE]>,
}

impl FileStream {
    pub fn from_std(file: std::fs::File) -> Self {
        let buffer = Box::new([0; BUFFER_SIZE]);
        let file = File::from_std(file);
        Self { file, buffer }
    }

    pub fn into_streamed_body(self) -> Body {
        Body::wrap_stream(self)
    }
}

impl Stream for FileStream {
    type Item = Result<Bytes, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Self {
            ref mut file,
            ref mut buffer,
        } = *self;
        let mut buf = tokio::io::ReadBuf::new(&mut buffer[..]);
        match Pin::new(file).poll_read(cx, &mut buf) {
            Poll::Ready(Ok(())) if buf.filled().is_empty() => Poll::Ready(None),
            Poll::Ready(Ok(())) => Poll::Ready(Some(Ok(Bytes::copy_from_slice(buf.filled())))),
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}
