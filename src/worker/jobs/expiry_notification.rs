use std::sync::Arc;

use crates_io_worker::BackgroundJob;
use diesel::{Connection as _, PgConnection};

use crate::{worker::Environment, Emails};

/// The threshold in days for the expiry notification.
const EXPIRY_THRESHOLD: i64 = 3;

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
        let mut conn = env.connection_pool.get()?;
        // Check if the token is expired
        // If the token is expired, trigger a notification
        check(&env.emails, &mut conn)
    }
}

// Check if the token is expired and trigger a notification if it is.
fn check(emails: &Emails, conn: &mut PgConnection) -> anyhow::Result<()> {
    info!("Checking if tokens are expired");
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
