use std::sync::Arc;

use crates_io_worker::BackgroundJob;
use diesel_async::AsyncPgConnection;
use typomania::Package;

use crate::Emails;
use crate::email::EmailMessage;
use crate::typosquat::{Cache, Crate};
use crate::worker::Environment;
use anyhow::Context;
use minijinja::context;
use tracing::{error, info};

/// A job to check the name of a newly published crate against the most popular crates to see if
/// the new crate might be typosquatting an existing, popular crate.
#[derive(Serialize, Deserialize, Debug)]
pub struct CheckTyposquat {
    name: String,
}

impl CheckTyposquat {
    pub fn new(name: &str) -> Self {
        Self { name: name.into() }
    }
}

impl BackgroundJob for CheckTyposquat {
    const JOB_NAME: &'static str = "check_typosquat";
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    #[instrument(skip(env), err)]
    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        let crate_name = self.name.clone();

        let mut conn = env.deadpool.get().await?;

        let cache = env.typosquat_cache(&mut conn).await?;
        check(&env.emails, cache, &mut conn, &crate_name).await
    }
}

async fn check(
    emails: &Emails,
    cache: &Cache,
    conn: &mut AsyncPgConnection,
    name: &str,
) -> anyhow::Result<()> {
    if let Some(harness) = cache.get_harness() {
        info!(name, "Checking new crate for potential typosquatting");

        let krate: Box<dyn Package> = Box::new(Crate::from_name(conn, name).await?);
        let squats = harness.check_package(name, krate)?;
        if !squats.is_empty() {
            // Well, well, well. For now, the only action we'll take is to e-mail people who
            // hopefully care to check into things more closely.
            info!(?squats, "Found potential typosquatting");

            let squats_data: Vec<_> = squats
                .iter()
                .map(|squat| {
                    context! {
                        display => squat.to_string(),
                        package => squat.package()
                    }
                })
                .collect();

            let email_context = context! {
                domain => emails.domain,
                crate_name => name,
                squats => squats_data
            };

            for recipient in cache.iter_emails() {
                if let Err(error) = send_notification_email(emails, recipient, &email_context).await
                {
                    error!(
                        ?error,
                        ?recipient,
                        "Failed to send possible typosquat notification"
                    );
                }
            }
        }
    }

    Ok(())
}

async fn send_notification_email(
    emails: &Emails,
    recipient: &str,
    context: &minijinja::Value,
) -> anyhow::Result<()> {
    let email = EmailMessage::from_template("possible_typosquat", context)
        .context("Failed to render email template")?;

    emails
        .send(recipient, email)
        .await
        .context("Failed to send email")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typosquat::test_util::faker;
    use crates_io_test_db::TestDatabase;
    use lettre::Address;

    #[tokio::test]
    async fn integration() -> anyhow::Result<()> {
        let emails = Emails::new_in_memory();
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        // Set up a user and a popular crate to match against.
        let user = faker::user(&mut conn, "a").await?;
        faker::crate_and_version(&mut conn, "my-crate", "It's awesome", &user, 100).await?;

        // Prime the cache so it only includes the crate we just created.
        let mut async_conn = test_db.async_connect().await;
        let cache = Cache::new(vec!["admin@example.com".to_string()], &mut async_conn).await?;
        let cache = Arc::new(cache);

        // Now we'll create new crates: one problematic, one not so.
        let other_user = faker::user(&mut async_conn, "b").await?;
        let angel = faker::crate_and_version(
            &mut async_conn,
            "innocent-crate",
            "I'm just a simple, innocent crate",
            &other_user,
            0,
        )
        .await?;
        let demon = faker::crate_and_version(
            &mut async_conn,
            "mycrate",
            "I'm even more innocent, obviously",
            &other_user,
            0,
        )
        .await?;

        // Run the check with a crate that shouldn't cause problems.
        check(&emails, &cache, &mut async_conn, &angel.name).await?;
        assert!(emails.mails_in_memory().await.unwrap().is_empty());

        // Now run the check with a less innocent crate.
        check(&emails, &cache, &mut async_conn, &demon.name).await?;
        let sent_mail = emails.mails_in_memory().await.unwrap();
        assert!(!sent_mail.is_empty());
        let sent = sent_mail.into_iter().next().unwrap();
        assert_eq!(&sent.0.to(), &["admin@example.com".parse::<Address>()?]);

        Ok(())
    }
}
