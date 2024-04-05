use std::sync::Arc;
use diesel::{Connection, PgConnection};

use crates_io_worker::BackgroundJob;
use crate::Emails;
use crate::worker::Environment;

/// The threshold in days for the expiry notification.
const EXPIRY_THRESHOLD: i64 = 3;

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
        let conn = env.deadpool.get().await?;
        conn.interact(move |conn| {
            // Check if the token is about to expire
            // If the token is about to expire, trigger a notification.
            check(&env.emails, conn)
        })
        .await
        .map_err(|err| anyhow!(err.to_string()))?
    }
}
// Check if the token is about to expire and send a notification if it is.
fn check(emails: &Emails, conn: &mut PgConnection) -> anyhow::Result<()> {
    info!("Checking if tokens are about to expire");
    let expired_tokens =
        crate::models::token::ApiToken::find_tokens_expiring_within_days(conn, EXPIRY_THRESHOLD)?;
    // Batch send notifications in transactions.
    const BATCH_SIZE: usize = 100;
    for chunk in expired_tokens.chunks(BATCH_SIZE) {
        conn.transaction(|conn| {
            for token in chunk {
                // Send notification.
            }
            Ok::<_, anyhow::Error>(())
        })?;
    }

    Ok(())
}
