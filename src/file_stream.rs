use std::task::{Context, Poll};
use std::{io::Error, pin::Pin};

use hyper::body::{Body, Bytes};
use tokio::{fs::File, io::AsyncRead, stream::Stream};

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
        match Pin::new(file).poll_read(cx, &mut buffer[..]) {
            Poll::Ready(Ok(0)) => Poll::Ready(None),
            Poll::Ready(Ok(size)) => Poll::Ready(Some(Ok(self.buffer[..size].to_owned().into()))),
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}
