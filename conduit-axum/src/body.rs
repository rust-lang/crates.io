use crate::file_stream::FileStream;
use axum::body::boxed;

pub fn conduit_into_axum(body: conduit::Body) -> axum::body::BoxBody {
    use conduit::Body::*;

    match body {
        Static(slice) => boxed(axum::body::Body::from(slice)),
        Owned(vec) => boxed(axum::body::Body::from(vec)),
        File(file) => boxed(FileStream::from_std(file).into_streamed_body()),
    }
}
