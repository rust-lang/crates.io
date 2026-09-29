//! This module contains all background jobs related to the git and
//! sparse indexes.

mod archive;
mod delete_archived_branch;
mod normalize;
mod squash;
mod sync;

pub use archive::ArchiveIndexBranch;
pub use delete_archived_branch::DeleteArchivedIndexBranch;
pub use normalize::NormalizeIndex;
pub use squash::SquashIndex;
pub use sync::{BulkSyncToGitIndex, SyncToGitIndex, SyncToSparseIndex};
