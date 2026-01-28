//! This module contains all background jobs related to the git and
//! sparse indexes.

mod normalize;
mod squash;
mod sync;

pub use normalize::NormalizeIndex;
pub use squash::SquashIndex;
pub use sync::{BulkSyncToGitIndex, SyncToGitIndex, SyncToSparseIndex};
