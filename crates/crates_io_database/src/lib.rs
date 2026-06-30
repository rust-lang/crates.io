#![doc = include_str!("../README.md")]

pub mod fns;
pub mod models;
mod pg_enum;
// Doc comments in `schema` are generated from the database's `COMMENT ON`
// strings, so they must not be reformatted to satisfy `doc_markdown`.
#[allow(clippy::doc_markdown)]
pub mod schema;
mod semver;
pub mod utils;

pub use self::semver::SemverVersion;
