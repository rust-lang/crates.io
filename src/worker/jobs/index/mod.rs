mod normalize;
mod squash;
mod sync;

pub use normalize::NormalizeIndex;
pub use squash::SquashIndex;
pub use sync::{SyncToGitIndex, SyncToSparseIndex};
