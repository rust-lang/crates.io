//! Structs using the builder pattern that make it easier to create records in tests.

mod dependency;
mod krate;
mod publish;
mod version;

pub use dependency::DependencyBuilder;
pub use krate::CrateBuilder;
pub use publish::PublishBuilder;
pub use version::VersionBuilder;
