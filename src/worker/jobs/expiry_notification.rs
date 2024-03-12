use std::sync::Arc;

use crates_io_worker::BackgroundJob;

use crate::worker::Environment;

/// A job responsible for monitoring the status of a token.
/// It checks if the token has reached its expiry date.
/// If the token is expired, the job triggers a notification.
#[derive(Default, Serialize, Deserialize, Debug)]
pub struct ExpiryNotification;

impl BackgroundJob for ExpiryNotification {
    const JOB_NAME: &'static str = "expiry_notification";

    type Context = Arc<Environment>;

    #[instrument(skip(env), err)]
    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        // Check if the token is expired
        // If the token is expired, trigger a notification
        Ok(())
    }
}
