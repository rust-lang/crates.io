use axum::body::boxed;

pub fn conduit_into_axum(body: conduit::Body) -> axum::body::BoxBody {
    use conduit::Body::*;

    match body {
        Static(slice) => boxed(axum::body::Body::from(slice)),
        Owned(vec) => boxed(axum::body::Body::from(vec)),
    }
}
