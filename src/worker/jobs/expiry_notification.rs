use std::sync::Arc;

use crates_io_worker::BackgroundJob;

use crate::worker::Environment;

/// A job responsible for monitoring the status of a token.
/// It checks if the token is about to reach its expiry date.
/// If the token is about to expire, the job triggers a notification.
#[derive(Default, Serialize, Deserialize, Debug)]
pub struct CheckAboutToExpireToken;

impl BackgroundJob for CheckAboutToExpireToken {
    const JOB_NAME: &'static str = "expiry_notification";

    type Context = Arc<Environment>;

    #[instrument(skip(env), err)]
    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        // Check if the token is about to expire
        // If the token is about to expire, trigger a notification.
        Ok(())
    }
}
