use crate::models::ApiToken;
use crate::schema::api_tokens;
use crate::{email::Email, models::User, worker::Environment, Emails};
use anyhow::anyhow;
use chrono::SecondsFormat;
use crates_io_worker::BackgroundJob;
use diesel::dsl::IntervalDsl;
use diesel::{
    dsl::now, BoolExpressionMethods, Connection, ExpressionMethods, NullableExpressionMethods,
    PgConnection, QueryDsl, QueryResult, RunQueryDsl, SelectableHelper,
};
use std::sync::Arc;

/// The threshold in days for the expiry notification.
const EXPIRY_THRESHOLD: chrono::TimeDelta = chrono::TimeDelta::days(3);

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
    let expired_tokens = find_tokens_expiring_within_days(conn, EXPIRY_THRESHOLD)?;
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
                match emails.send(&recipient, email) {
                    Ok(_) => {
                        // Update the token to prevent duplicate notifications.
                        diesel::update(token)
                            .set(api_tokens::expiry_notification_at.eq(now.nullable()))
                            .execute(conn)?;
                    }
                    Err(e) => {
                        error!("Failed to send email: {:?} to {}", e, recipient);
                    }
                }
            }
            Ok::<_, anyhow::Error>(())
        })?;
    }

    Ok(())
}

/// Find all tokens that are not revoked and will expire within the specified number of days.
pub fn find_tokens_expiring_within_days(
    conn: &mut PgConnection,
    days_until_expiry: chrono::TimeDelta,
) -> QueryResult<Vec<ApiToken>> {
    api_tokens::table
        .filter(api_tokens::revoked.eq(false))
        .filter(
            api_tokens::expired_at
                .is_not_null()
                .and(api_tokens::expired_at.assume_not_null().gt(now)) // Ignore already expired tokens
                .and(
                    api_tokens::expired_at
                        .assume_not_null()
                        .lt(now + days_until_expiry.num_days().day()),
                ),
        )
        .filter(api_tokens::expiry_notification_at.is_null())
        .select(ApiToken::as_select())
        .get_results(conn)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::NewUser;
    use crate::{
        models::token::ApiToken, schema::api_tokens, test_util::test_db_connection,
        util::token::PlainToken,
    };
    use diesel::dsl::IntervalDsl;
    use diesel::{QueryDsl, SelectableHelper};
    use lettre::Address;

    #[tokio::test]
    async fn test_expiry_notification() -> anyhow::Result<()> {
        let emails = Emails::new_in_memory();
        let (_test_db, mut conn) = test_db_connection();

        // Set up a user and a token that is about to expire.
        let user = NewUser::new(0, "a", None, None, "token").create_or_update(
            Some("testuser@test.com"),
            &Emails::new_in_memory(),
            &mut conn,
        )?;
        let token = PlainToken::generate();

        let token: ApiToken = diesel::insert_into(api_tokens::table)
            .values((
                api_tokens::user_id.eq(user.id),
                api_tokens::name.eq("test_token"),
                api_tokens::token.eq(token.hashed()),
                api_tokens::expired_at.eq(now.nullable() + (EXPIRY_THRESHOLD.num_days() - 1).day()),
            ))
            .returning(ApiToken::as_returning())
            .get_result(&mut conn)?;

        // Insert a few tokens that are not set to expire.
        let not_expired_offset = EXPIRY_THRESHOLD.num_days() + 1;
        for i in 0..3 {
            let token = PlainToken::generate();
            diesel::insert_into(api_tokens::table)
                .values((
                    api_tokens::user_id.eq(user.id),
                    api_tokens::name.eq(format!("test_token{i}")),
                    api_tokens::token.eq(token.hashed()),
                    api_tokens::expired_at.eq(now.nullable() + (not_expired_offset).day()),
                ))
                .returning(ApiToken::as_returning())
                .get_result(&mut conn)?;
        }

        // Check that the token is about to expire.
        check(&emails, &mut conn)?;

        // Check that an email was sent.
        let sent_mail = emails.mails_in_memory().unwrap();
        assert_eq!(sent_mail.len(), 1);
        let sent = &sent_mail[0];
        assert_eq!(&sent.0.to(), &["testuser@test.com".parse::<Address>()?]);
        assert!(sent.1.contains("Your token is about to expire"));
        let updated_token = api_tokens::table
            .filter(api_tokens::id.eq(token.id))
            .filter(api_tokens::expiry_notification_at.is_not_null())
            .select(ApiToken::as_select())
            .first::<ApiToken>(&mut conn)?;
        assert_eq!(updated_token.name, "test_token".to_owned());

        // Check that the token is not about to expire.
        let tokens = api_tokens::table
            .filter(api_tokens::revoked.eq(false))
            .filter(api_tokens::expiry_notification_at.is_null())
            .select(ApiToken::as_select())
            .load::<ApiToken>(&mut conn)?;
        assert_eq!(tokens.len(), 3);

        // Insert a already expired token.
        let token = PlainToken::generate();
        diesel::insert_into(api_tokens::table)
            .values((
                api_tokens::user_id.eq(user.id),
                api_tokens::name.eq("expired_token"),
                api_tokens::token.eq(token.hashed()),
                api_tokens::expired_at.eq(diesel::dsl::now.nullable() - 1.day()),
            ))
            .returning(ApiToken::as_returning())
            .get_result(&mut conn)?;

        // Check that the token is not about to expire.
        check(&emails, &mut conn)?;

        // Check that no email was sent.
        let sent_mail = emails.mails_in_memory().unwrap();
        assert_eq!(sent_mail.len(), 1);

        Ok(())
    }
}
