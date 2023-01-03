use crate::ConduitFallback;

use std::future::Future;
use std::net::SocketAddr;

/// A builder for a `hyper::Server` (behind an opaque `impl Future`).
#[derive(Debug)]
pub struct Server;

impl Server {
    /// Bind a handler to an address.
    ///
    /// This returns an opaque `impl Future` so while it can be directly spawned on a
    /// `tokio::Runtime` it is not possible to furter configure the `hyper::Server`.  If more
    /// control, such as configuring a graceful shutdown is necessary, then call
    /// `Service::from_blocking` instead.
    pub fn serve<H: conduit::Handler>(
        addr: &SocketAddr,
        router: axum::Router,
        handler: H,
    ) -> impl Future {
        let router = router.conduit_fallback(handler);
        let make_service = router.into_make_service_with_connect_info::<SocketAddr>();

        hyper::Server::bind(addr).serve(make_service)
    }
}
