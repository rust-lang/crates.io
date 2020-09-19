//! Serve the Ember.js frontend HTML
//!
//! Paths intended for the inner `api_handler` are passed along to the remaining middleware layers
//! as normal. Requests not intended for the backend will be served HTML to boot the Ember.js
//! frontend. During local development, if so configured, these requests will instead be proxied to
//! Ember FastBoot (`node ./fastboot.js`).
//!
//! For now, there is an additional check to see if the `Accept` header contains "html". This is
//! likely to be removed in the future.

use super::prelude::*;
use std::fmt::Write;

use crate::util::{errors::NotFound, AppResponse};

use anyhow::{ensure, Result};
use conduit::{Body, HandlerResult};
use conduit_static::Static;
use reqwest::blocking::Client;

pub(super) struct EmberHtml {
    api_handler: Option<Box<dyn Handler>>,
    static_handler: Static,
    fastboot_client: Option<Client>,
}

impl EmberHtml {
    pub fn new(path: &str) -> Self {
        let fastboot_client = match dotenv::var("USE_FASTBOOT") {
            Ok(val) if val == "staging-experimental" => Some(Client::new()),
            _ => None,
        };

        Self {
            api_handler: None,
            static_handler: Static::new(path),
            fastboot_client,
        }
    }
}

impl AroundMiddleware for EmberHtml {
    fn with_handler(&mut self, handler: Box<dyn Handler>) {
        self.api_handler = Some(handler);
    }
}

impl Handler for EmberHtml {
    fn call(&self, req: &mut dyn RequestExt) -> HandlerResult {
        let api_handler = self.api_handler.as_ref().unwrap();

        // The "/git/" prefix is only used in development (when within a docker container)
        if req.path().starts_with("/api/") || req.path().starts_with("/git/") {
            api_handler.call(req)
        } else {
            if let Some(client) = &self.fastboot_client {
                // During local fastboot development, forward requests to the local fastboot server.
                // In prodution, including when running with fastboot, nginx proxies the requests
                // to the correct endpoint and requests should never make it here.
                return proxy_to_fastboot(client, req).map_err(From::from);
            }

            if req
                .headers()
                .get_all(header::ACCEPT)
                .iter()
                .any(|val| val.to_str().unwrap_or_default().contains("html"))
            {
                // Serve static Ember page to bootstrap the frontend
                *req.path_mut() = String::from("/index.html");
                self.static_handler.call(req)
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
fn proxy_to_fastboot(client: &Client, req: &mut dyn RequestExt) -> Result<AppResponse> {
    ensure!(
        req.method() == conduit::Method::GET,
        "Only support GET but request method was {}",
        req.method()
    );

    let mut url = format!("http://127.0.0.1:9000{}", req.path());
    if let Some(query) = req.query_string() {
        write!(url, "?{}", query)?;
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
    Ok(builder.body(Body::from_vec(body))?)
}
