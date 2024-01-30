//! This module should contain all tests that test a single webserver route.
//!
//! Each `/api/v1` (or `/api/private`) sub-API should have its own module, with
//! submodules divided by the specific endpoint (e.g. `list`, `create`, `read`,
//! `update`, `delete`).
//!
//! ## Examples
//!
//! - testing all the ways authentication works or fails on a specific route
//! - testing error behavior of a route
//! - testing output serialization of a route
//! - testing query parameter combinations of a route

pub mod categories;
pub mod category_slugs;
pub mod crates;
pub mod keywords;
pub mod me;
pub mod metrics;
mod private;
pub mod session;
pub mod summary;
pub mod users;
