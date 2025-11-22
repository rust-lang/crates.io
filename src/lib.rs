//! This crate implements the backend server for <https://crates.io/>
//!
//! All implemented routes are defined in the [middleware](fn.middleware.html) function and
//! implemented in the [category](category/index.html), [keyword](keyword/index.html),
//! [krate](krate/index.html), [user](user/index.html) and [version](version/index.html) modules.

pub use crate::{app::App, email::Emails};
pub use crates_io_api_types as views;
pub use crates_io_database::{models, schema};
use std::sync::Arc;

use crate::app::AppState;
use crate::router::build_axum_router;
use tikv_jemallocator::Jemalloc;

#[global_allocator]
static ALLOC: Jemalloc = Jemalloc;

pub mod app;
pub mod auth;
pub mod boot;
pub mod certs;
pub mod cloudfront;
pub mod config;
pub mod controllers;
pub mod db;
pub mod email;
pub mod headers;
pub mod index;
mod licenses;
pub mod metrics;
pub mod middleware;
pub mod openapi;
pub mod rate_limiter;
mod router;
pub mod sentry;
pub mod sqs;
pub mod ssh;
pub mod storage;
pub mod tasks;
pub mod typosquat;
pub mod util;
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
