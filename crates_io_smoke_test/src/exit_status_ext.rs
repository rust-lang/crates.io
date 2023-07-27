//! This is a backport of the unstable "exit_status_error" std library feature.

use std::process::ExitStatus;

#[derive(Debug, thiserror::Error)]
#[error("process exited unsuccessfully: {0}")]
pub struct ExitStatusError(ExitStatus);

pub trait ExitStatusExt {
    fn exit_ok(&self) -> Result<(), ExitStatusError>;
}

impl ExitStatusExt for ExitStatus {
    fn exit_ok(&self) -> Result<(), ExitStatusError> {
        match self.success() {
            true => Ok(()),
            false => Err(ExitStatusError(*self)),
        }
    }
}
