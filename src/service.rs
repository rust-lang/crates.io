use super::adaptor::{ConduitRequest, RequestInfo};

use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use futures::prelude::*;
use hyper::{service, Body, Error, Request, Response, StatusCode};
use tracing::error;

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
    /// # use conduit_hyper::Service;
    /// # use std::{error, io};
    /// # use conduit::{Handler, Request, Response};
    /// #
    /// # struct Endpoint();
    /// # impl Handler for Endpoint {
    /// #     fn call(&self, _: &mut dyn Request) -> Result<Response, Box<dyn error::Error + Send>> {
    /// #         Ok(Response {
    /// #             status: (200, "OK"),
    /// #             headers: Default::default(),
    /// #             body: Box::new(io::Cursor::new("")),
    /// #         })
    /// #     }
    /// # }
    /// # let app = Endpoint();
    /// let handler = Arc::new(app);
    /// let make_service =
    ///     hyper::service::make_service_fn(move |socket: &hyper::server::conn::AddrStream| {
    ///         let addr = socket.remote_addr();
    ///         let handler = handler.clone();
    ///         async move { Service::from_conduit(handler, addr) }
    ///     });
    ///
    /// # let port = 0;
    /// let addr = ([127, 0, 0, 1], port).into();
    /// let server = hyper::Server::bind(&addr).serve(make_service);
    /// ```
    pub fn from_conduit<H: conduit::Handler>(
        handler: Arc<H>,
        remote_addr: SocketAddr,
    ) -> Result<
        impl tower_service::Service<
            Request<Body>,
            Response = Response<Body>,
            Error = Error,
            Future = impl Future<Output = Result<Response<Body>, Error>> + Send + 'static,
        >,
        Error,
    > {
        Ok(service::service_fn(move |request: Request<Body>| {
            blocking_handler(handler.clone(), request, remote_addr)
        }))
    }
}

// pub(crate) is for tests
pub(crate) async fn blocking_handler<H: conduit::Handler>(
    handler: Arc<H>,
    request: Request<Body>,
    remote_addr: SocketAddr,
) -> Result<Response<Body>, Error> {
    let (parts, body) = request.into_parts();
    let full_body = body.try_concat().await?;
    let mut request_info = RequestInfo::new(parts, full_body);

    let future = future::poll_fn(move |_| {
        tokio_executor::threadpool::blocking(|| {
            let mut request = ConduitRequest::new(&mut request_info, remote_addr);
            handler
                .call(&mut request)
                .map(good_response)
                .unwrap_or_else(|e| error_response(&e.to_string()))
        })
        .map_err(|_| panic!("The threadpool shut down"))
    });

    // Spawn as a new top-level task, otherwise the parent task is blocked as well
    let (future, handle) = future.remote_handle();
    tokio_executor::spawn(future);
    handle.await
}

/// Builds a `hyper::Response` given a `conduit:Response`
fn good_response(mut response: conduit::Response) -> Response<Body> {
    let mut body = Vec::new();
    if response.body.write_body(&mut body).is_err() {
        return error_response("Error writing body");
    }

    let mut builder = Response::builder();
    let status = match StatusCode::from_u16(response.status.0 as u16) {
        Ok(s) => s,
        Err(e) => return error_response(&e.to_string()),
    };
    builder.status(status);

    for (key, values) in response.headers {
        for value in values {
            builder.header(key.as_str(), value.as_str());
        }
    }

    builder
        .body(body.into())
        .unwrap_or_else(|e| error_response(&e.to_string()))
}

/// Logs an error message and returns a generic status 500 response
fn error_response(message: &str) -> Response<Body> {
    error!("Internal Server Error: {}", message);
    let body = Body::from("Internal Server Error");
    Response::builder()
        .status(500)
        .body(body)
        .expect("Unexpected invalid header")
}
