use crate::models::ApiToken;
use crate::{email::Email, models::User, worker::Environment, Emails};
use anyhow::anyhow;
use chrono::SecondsFormat;
use crates_io_worker::BackgroundJob;
use diesel::{
    dsl::now, Connection, ExpressionMethods, NullableExpressionMethods, PgConnection, RunQueryDsl,
};
use std::sync::Arc;

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
    let expired_tokens = ApiToken::find_tokens_expiring_within_days(conn, EXPIRY_THRESHOLD)?;
    // Batch send notifications in transactions.
    const BATCH_SIZE: usize = 100;
    for chunk in expired_tokens.chunks(BATCH_SIZE) {
        conn.transaction(|conn| {
            for token in chunk {
                // Send notification.
                let user = User::find(conn, token.user_id)?;
                let Some(recipient) = user.email(conn)? else {
                    return Err(anyhow!("No address found"));
                };
                let email = ExpiryNotificationEmail {
                    name: &user.gh_login,
                    token_name: &token.name,
                    expiry_date: token.expired_at.unwrap().and_utc(),
                };
                emails.send(&recipient, email)?;
                // Also update the token to prevent duplicate notifications.
                diesel::update(token)
                    .set(crate::schema::api_tokens::expiry_notification_at.eq(now.nullable()))
                    .execute(conn)?;
            }
            Ok::<_, anyhow::Error>(())
        })?;
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct ExpiryNotificationEmail<'a> {
    name: &'a str,
    token_name: &'a str,
    expiry_date: chrono::DateTime<chrono::Utc>,
}

impl<'a> Email for ExpiryNotificationEmail<'a> {
    const SUBJECT: &'static str = "Your token is about to expire";

    fn body(&self) -> String {
        format!(
            r#"Hi {},

We noticed your token "{}" will expire on {}.

If this token is still needed, visit https://crates.io/settings/tokens/new to generate a new one.

Thanks,
The crates.io team"#,
            self.name,
            self.token_name,
            self.expiry_date.to_rfc3339_opts(SecondsFormat::Secs, true)
        )
    }
}
