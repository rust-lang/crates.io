//! This crate implements the backend server for <https://crates.io/>
//!
//! All implemented routes are defined in the [middleware](fn.middleware.html) function and
//! implemented in the [category](category/index.html), [keyword](keyword/index.html),
//! [krate](krate/index.html), [user](user/index.html) and [version](version/index.html) modules.

#![warn(clippy::all, rust_2018_idioms)]
// `diesel` macros are currently generating code that breaks this rule, so
// we have to disable it for now.
#![allow(clippy::extra_unused_lifetimes)]

#[cfg(test)]
#[macro_use]
extern crate claims;
#[macro_use]
extern crate derive_deref;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate tracing;

pub use crate::{app::App, email::Emails, uploaders::Uploader};
use std::str::FromStr;
use std::sync::Arc;

use conduit_axum::ConduitFallback;
use tikv_jemallocator::Jemalloc;

#[global_allocator]
static ALLOC: Jemalloc = Jemalloc;

pub mod admin;
mod app;
pub mod background_jobs;
pub mod boot;
pub mod config;
pub mod db;
mod downloads_counter;
pub mod email;
pub mod github;
pub mod headers;
pub mod metrics;
pub mod middleware;
mod publish_rate_limit;
pub mod schema;
pub mod sql;
mod test_util;
pub mod uploaders;
pub mod util;
pub mod worker;

pub mod auth;
pub mod controllers;
pub mod models;
mod router;
pub mod sentry;
pub mod views;

/// Used for setting different values depending on whether the app is being run in production,
/// in development, or for testing.
///
/// The app's `config.env` value is set in *src/bin/server.rs* to `Production` if the environment
/// variable `HEROKU` is set and `Development` otherwise. `config.env` is set to `Test`
/// unconditionally in *src/test/all.rs*.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Env {
    Development,
    Test,
    Production,
}

/// Configures routes, sessions, logging, and other middleware.
///
/// Called from *src/bin/server.rs*.
pub fn build_handler(app: Arc<App>) -> axum::Router {
    use crate::middleware::debug_requests;
    use crate::middleware::log_request::log_requests;
    use ::sentry::integrations::tower as sentry_tower;
    use axum::error_handling::HandleErrorLayer;
    use axum::middleware::from_fn;

    let env = app.config.env();

    let endpoints = router::build_router(&app);
    let conduit_handler = middleware::build_middleware(app, endpoints);

    type Request = http::Request<axum::body::Body>;

    let middleware = tower::ServiceBuilder::new()
        .layer(sentry_tower::NewSentryLayer::<Request>::new_from_top())
        .layer(sentry_tower::SentryHttpLayer::with_transaction())
        .layer(from_fn(log_requests))
        // The following layer is unfortunately necessary for `option_layer()` to work
        .layer(HandleErrorLayer::new(dummy_error_handler))
        // Optionally print debug information for each request
        // To enable, set the environment variable: `RUST_LOG=cargo_registry::middleware=debug`
        .option_layer((env == Env::Development).then(|| from_fn(debug_requests)));

    axum::Router::new()
        .conduit_fallback(conduit_handler)
        .layer(middleware)
}

/// This function is only necessary because `.option_layer()` changes the error type
/// and we need to change it back. Since the axum middleware has no way of returning
/// an actual error this function should never actually be called.
async fn dummy_error_handler(_err: axum::BoxError) -> http::StatusCode {
    http::StatusCode::INTERNAL_SERVER_ERROR
}

/// Convenience function requiring that an environment variable is set.
///
/// Ensures that we've initialized the dotenv crate in order to read environment variables
/// from a *.env* file if present. Don't use this for optionally set environment variables.
///
/// # Panics
///
/// Panics if the environment variable with the name passed in as an argument is not defined
/// in the current environment.
#[track_caller]
pub fn env(s: &str) -> String {
    dotenv::var(s).unwrap_or_else(|_| panic!("must have `{}` defined", s))
}

/// Parse an optional environment variable
///
/// Ensures that we've initialized the dotenv crate in order to read environment variables
/// from a *.env* file if present. A variable that is set to invalid unicode will be handled
/// as if it was unset.
///
/// # Panics
///
/// Panics if the environment variable is set but cannot be parsed as the requested type.
#[track_caller]
pub fn env_optional<T: FromStr>(s: &str) -> Option<T> {
    dotenv::var(s).ok().map(|s| {
        s.parse()
            .unwrap_or_else(|_| panic!("`{}` was defined but could not be parsed", s))
    })
}
