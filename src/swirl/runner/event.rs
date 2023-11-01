use diesel::result::Error as DieselError;

use crate::db::PoolError;

#[derive(Debug)]
pub(super) enum Event {
    Working,
    NoJobAvailable,
    ErrorLoadingJob(DieselError),
    FailedToAcquireConnection(PoolError),
}
