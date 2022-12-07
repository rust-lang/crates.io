use tokio::task::JoinError;

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error(transparent)]
    JoinError(#[from] JoinError),
    #[error(transparent)]
    Hyper(#[from] hyper::Error),
}
