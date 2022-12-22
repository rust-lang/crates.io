use std::fmt;

use diesel::result::Error as DieselError;

use crate::db::PoolError;

pub(super) enum Event {
    Working,
    NoJobAvailable,
    ErrorLoadingJob(DieselError),
    FailedToAcquireConnection(PoolError),
}

impl fmt::Debug for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Event::Working => f.debug_struct("Working").finish(),
            Event::NoJobAvailable => f.debug_struct("NoJobAvailable").finish(),
            Event::ErrorLoadingJob(e) => f.debug_tuple("ErrorLoadingJob").field(e).finish(),
            Event::FailedToAcquireConnection(e) => {
                f.debug_tuple("FailedToAcquireConnection").field(e).finish()
            }
        }
    }
}
