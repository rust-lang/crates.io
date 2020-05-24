use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use hyper::{service, Body, Request, Response};

mod blocking_handler;
mod error;

pub use blocking_handler::BlockingHandler;
pub use error::ServiceError;

/// A builder for a `hyper::Service`.
#[derive(Debug)]
pub struct Service;

impl Service {
    /// Turn a conduit handler into a `Service` which can be bound to a `hyper::Server`.
    ///
    /// The returned service can be built into a `hyper::Server` using `make_service_fn` and
    /// capturing the socket `remote_addr`.
    ///
    /// ```no_run
    /// # use std::sync::Arc;
    /// # use conduit_hyper::{BlockingHandler, Service};
    /// # use conduit::{box_error, Body, Handler, HandlerResult, RequestExt, Response};
    /// #
    /// # struct Endpoint();
    /// # impl Handler for Endpoint {
    /// #     fn call(&self, _: &mut dyn RequestExt) -> HandlerResult {
    /// #         Response::builder().body(Body::empty()).map_err(box_error)
    /// #     }
    /// # }
    /// # let app = Endpoint();
    /// let handler = Arc::new(BlockingHandler::new(app));
    /// let make_service =
    ///     hyper::service::make_service_fn(move |socket: &hyper::server::conn::AddrStream| {
    ///         let addr = socket.remote_addr();
    ///         let handler = handler.clone();
    ///         async move { Service::from_blocking(handler, addr) }
    ///     });
    ///
    /// # let port = 0;
    /// let addr = ([127, 0, 0, 1], port).into();
    /// let server = hyper::Server::bind(&addr).serve(make_service);
    /// ```
    pub fn from_blocking<H: conduit::Handler>(
        handler: Arc<BlockingHandler<H>>,
        remote_addr: SocketAddr,
    ) -> Result<
        impl tower_service::Service<
            Request<Body>,
            Response = Response<Body>,
            Error = ServiceError,
            Future = impl Future<Output = Result<Response<Body>, ServiceError>> + Send + 'static,
        >,
        ServiceError,
    > {
        Ok(service::service_fn(move |request: Request<Body>| {
            handler.clone().blocking_handler(request, remote_addr)
        }))
    }
}
