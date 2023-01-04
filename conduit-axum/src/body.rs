use axum::body::{boxed, Body, Bytes};

pub fn conduit_into_axum(body: Bytes) -> axum::body::BoxBody {
    boxed(Body::from(body))
}
