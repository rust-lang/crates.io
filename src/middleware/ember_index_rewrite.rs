//! Rewrite the request path to "index.html" if the path doesn't start
//! with "/api" and the Accept header contains "html".

use super::prelude::*;
use std::fmt::Write;

use crate::util::{errors::NotFound, AppResponse, Error, RequestProxy};

use conduit::{Body, HandlerResult};
use reqwest::blocking::Client;

// Can't derive debug because of Handler and Static.
#[allow(missing_debug_implementations)]
pub struct EmberIndexRewrite {
    handler: Option<Box<dyn Handler>>,
    fastboot_client: Option<Client>,
}

impl Default for EmberIndexRewrite {
    fn default() -> EmberIndexRewrite {
        let fastboot_client = match dotenv::var("USE_FASTBOOT") {
            Ok(val) if val == "staging-experimental" => Some(Client::new()),
            _ => None,
        };

        EmberIndexRewrite {
            handler: None,
            fastboot_client,
        }
    }
}

impl AroundMiddleware for EmberIndexRewrite {
    fn with_handler(&mut self, handler: Box<dyn Handler>) {
        self.handler = Some(handler);
    }
}

impl Handler for EmberIndexRewrite {
    fn call(&self, req: &mut dyn RequestExt) -> HandlerResult {
        let handler = self.handler.as_ref().unwrap();

        // The "/git/" prefix is only used in development (when within a docker container)
        if req.path().starts_with("/api/") || req.path().starts_with("/git/") {
            handler.call(req)
        } else {
            if let Some(client) = &self.fastboot_client {
                // During local fastboot development, forward requests to the local fastboot server.
                // In prodution, including when running with fastboot, nginx proxies the requests
                // to the correct endpoint and requests should never make it here.
                return proxy_to_fastboot(client, req).map_err(box_error);
            }

            if req
                .headers()
                .get_all(header::ACCEPT)
                .iter()
                .any(|val| val.to_str().unwrap_or_default().contains("html"))
            {
                // Serve static Ember page to bootstrap the frontend
                handler.call(&mut RequestProxy::rewrite_path(req, "/index.html"))
            } else {
                // Return a 404 to crawlers that don't send `Accept: text/hml`.
                // This is to preserve legacy behavior and will likely change.
                // Most of these crawlers probably won't execute our frontend JS anyway, but
                // it would be nice to bootstrap the app for crawlers that do execute JS.
                Ok(NotFound.into())
            }
        }
    }
}

/// Proxy to the fastboot server in development mode
///
/// This handler is somewhat hacky, and is not intended for usage in production.
///
/// # Panics
///
/// This function can panic and should only be used in development mode.
fn proxy_to_fastboot(client: &Client, req: &mut dyn RequestExt) -> Result<AppResponse, Error> {
    if req.method() != conduit::Method::GET {
        return Err(format!("Only support GET but request method was {}", req.method()).into());
    }

    let mut url = format!("http://127.0.0.1:9000{}", req.path());
    if let Some(query) = req.query_string() {
        write!(url, "?{}", query).map_err(|e| e.to_string())?;
    }
    let mut fastboot_response = client
        .request(req.method().into(), &*url)
        .headers(req.headers().clone())
        .send()?;
    let mut body = Vec::new();
    fastboot_response.copy_to(&mut body)?;

    let mut builder = Response::builder().status(fastboot_response.status());
    builder
        .headers_mut()
        .unwrap()
        .extend(fastboot_response.headers().clone());
    builder.body(Body::from_vec(body)).map_err(Into::into)
}
