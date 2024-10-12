#[macro_use]
extern crate serde;
#[macro_use]
extern crate tracing;

mod credentials;
mod data;
pub mod features;
mod repo;
mod ser;
#[cfg(feature = "testing")]
pub mod testing;

pub use crate::credentials::Credentials;
pub use crate::data::{Crate, Dependency, DependencyKind};
pub use crate::repo::{Repository, RepositoryConfig};
pub use crate::ser::write_crates;
