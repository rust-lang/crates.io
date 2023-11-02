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
extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate tracing;

pub use crate::{app::App, email::Emails};
use std::str::FromStr;
use std::sync::Arc;

use crate::app::AppState;
use crate::router::build_axum_router;
use tikv_jemallocator::Jemalloc;

#[global_allocator]
static ALLOC: Jemalloc = Jemalloc;

pub mod admin;
mod app;
pub mod auth;
pub mod boot;
pub mod cloudfront;
pub mod config;
pub mod controllers;
pub mod db;
mod downloads_counter;
pub mod email;
pub mod fastly;
pub mod github;
pub mod headers;
mod licenses;
pub mod metrics;
pub mod middleware;
pub mod models;
pub mod rate_limiter;
mod real_ip;
mod router;
pub mod schema;
pub mod sentry;
pub mod sql;
pub mod ssh;
pub mod storage;
mod test_util;
pub mod util;
pub mod views;
pub mod worker;

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
    let state = AppState(app);

    let axum_router = build_axum_router(state.clone());
    middleware::apply_axum_middleware(state, axum_router)
}

/// Convenience function requiring that an environment variable is set.
///
/// Ensures that we've initialized the dotenvy crate in order to read environment variables
/// from a *.env* file if present. Don't use this for optionally set environment variables.
///
/// # Panics
///
/// Panics if the environment variable with the name passed in as an argument is not defined
/// in the current environment.
#[track_caller]
pub fn env(s: &str) -> String {
    dotenvy::var(s).unwrap_or_else(|_| panic!("must have `{s}` defined"))
}

/// Parse an optional environment variable
///
/// Ensures that we've initialized the dotenvy crate in order to read environment variables
/// from a *.env* file if present. A variable that is set to invalid unicode will be handled
/// as if it was unset.
///
/// # Panics
///
/// Panics if the environment variable is set but cannot be parsed as the requested type.
#[track_caller]
pub fn env_optional<T: FromStr>(s: &str) -> Option<T> {
    dotenvy::var(s).ok().map(|s| {
        s.parse()
            .unwrap_or_else(|_| panic!("`{s}` was defined but could not be parsed"))
    })
}
