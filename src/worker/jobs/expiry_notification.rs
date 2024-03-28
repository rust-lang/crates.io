use std::sync::Arc;

use anyhow::anyhow;
use crates_io_worker::BackgroundJob;
use diesel::{
    dsl::now, Connection, ExpressionMethods, NullableExpressionMethods, PgConnection, RunQueryDsl,
};

use crate::{email::Email, models::User, worker::Environment, Emails};

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
                let user = User::find(conn, token.user_id)?;
                let Some(recipient) = user.email(conn)? else {
                    return Err(anyhow!("No address found"));
                };
                let email = ExpiryNotificationEmail {
                    name: user.gh_login.clone(),
                    token_name: token.name.clone(),
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
struct ExpiryNotificationEmail {
    name: String,
    token_name: String,
}

impl Email for ExpiryNotificationEmail {
    const SUBJECT: &'static str = "Your token is about to expire";

    fn body(&self) -> String {
        format!(
            r#"Hi @{name},

    We noticed your token "{token_name}" will expire in about {EXPIRY_THRESHOLD} days.

    If this token is still needed, visit https://crates.io/settings/tokens/new to generate a new one.

    Thanks,
    The crates.io team"#,
            name = self.name,
            token_name = self.token_name,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::token::ApiToken, schema::api_tokens, test_util::test_db_connection,
        typosquat::test_util::Faker, util::token::PlainToken,
    };
    use diesel::{QueryDsl, SelectableHelper};
    use lettre::Address;

    #[tokio::test]
    async fn test_expiry_notification() -> anyhow::Result<()> {
        let emails = Emails::new_in_memory();
        let (_test_db, mut conn) = test_db_connection();
        let mut faker = Faker::new();

        // Set up a user and a token that is about to expire.
        let user = faker.user(&mut conn, "a", Some("testuser@test.com".to_owned()))?;
        let token = PlainToken::generate();
        let expired_at = diesel::dsl::now;

        let token: ApiToken = diesel::insert_into(api_tokens::table)
            .values((
                api_tokens::user_id.eq(user.id),
                api_tokens::name.eq("test_token"),
                api_tokens::token.eq(token.hashed()),
                api_tokens::expired_at.eq(expired_at),
            ))
            .returning(ApiToken::as_returning())
            .get_result(&mut conn)?;

        // Check that the token is about to expire.
        check(&emails, &mut conn)?;

        // Check that an email was sent.
        let sent_mail = emails.mails_in_memory().unwrap();
        assert_eq!(sent_mail.len(), 1);
        let sent = &sent_mail[0];
        assert_eq!(&sent.0.to(), &["testuser@test.com".parse::<Address>()?]);
        assert!(sent.1.contains("Your token is about to expire"));
        let update_token = api_tokens::table
            .filter(api_tokens::id.eq(token.id))
            .select(ApiToken::as_select())
            .first::<ApiToken>(&mut conn)?;
        assert!(update_token.expiry_notification_at.is_some());

        Ok(())
    }
}
