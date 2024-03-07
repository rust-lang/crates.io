/// An error occurred queueing the job
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EnqueueError {
    /// An error occurred serializing the job
    #[error(transparent)]
    SerializationError(#[from] serde_json::error::Error),

    /// An error occurred inserting the job into the database
    #[error(transparent)]
    DatabaseError(#[from] diesel::result::Error),
}
