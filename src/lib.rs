//! This crate implements the backend server for <https://crates.io/>
//!
//! All implemented routes are defined in the [middleware](fn.middleware.html) function and
//! implemented in the [category](category/index.html), [keyword](keyword/index.html),
//! [krate](krate/index.html), [user](user/index.html) and [version](version/index.html) modules.
#![deny(warnings)]
#![deny(missing_debug_implementations, missing_copy_implementations)]
#![deny(bare_trait_objects)]
#![recursion_limit = "256"]
#![allow(unknown_lints, proc_macro_derive_resolution_fallback)] // TODO: This can be removed after diesel-1.4

#[macro_use]
extern crate derive_deref;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

pub use crate::{app::App, config::Config, uploaders::Uploader};
use std::sync::Arc;

use conduit_middleware::MiddlewareBuilder;

mod app;
pub mod boot;
mod config;
pub mod db;
pub mod email;
pub mod git;
pub mod github;
pub mod middleware;
pub mod render;
pub mod schema;
pub mod uploaders;
pub mod util;

pub mod controllers;
pub mod models;
mod router;
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

/// Used for setting different values depending on the type of registry this instance is.
///
/// `Primary` indicates this instance is a primary registry that is the source of truth for these
/// crates' information. `ReadOnlyMirror` indicates this instanceis a read-only mirror of crate
/// information that exists on another instance.
///
/// The app's `config.mirror` value is set in *src/bin/server.rs* to `ReadOnlyMirror` if the
/// `MIRROR` environment variable is set and to `Primary` otherwise.
///
/// There may be more ways to run crates.io servers in the future, such as a
/// mirror that also has private crates that crates.io does not have.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Replica {
    Primary,
    ReadOnlyMirror,
}

/// Configures routes, sessions, logging, and other middleware.
///
/// Called from *src/bin/server.rs*.
pub fn build_handler(app: Arc<App>) -> MiddlewareBuilder {
    let endpoints = router::build_router(&app);
    middleware::build_middleware(app, endpoints)
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
pub fn env(s: &str) -> String {
    dotenv::dotenv().ok();
    ::std::env::var(s).unwrap_or_else(|_| panic!("must have `{}` defined", s))
}

sql_function!(fn lower(x: ::diesel::sql_types::Text) -> ::diesel::sql_types::Text);
