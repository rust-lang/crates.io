use crate::{BlockingHandler, Service};

use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use hyper::server::conn::AddrStream;
use hyper::service;
use service::make_service_fn;

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
        handler: H,
        max_threads: usize,
    ) -> impl Future {
        let handler = Arc::new(BlockingHandler::new(handler, max_threads));
        let make_service = make_service_fn(move |socket: &AddrStream| {
            let handler = handler.clone();
            let remote_addr = socket.remote_addr();
            async move { Service::from_blocking(handler, remote_addr) }
        });

        hyper::Server::bind(&addr).serve(make_service)
    }
}
