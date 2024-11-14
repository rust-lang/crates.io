use std::sync::Arc;

use crates_io_worker::BackgroundJob;
use diesel_async::AsyncPgConnection;
use typomania::Package;

use crate::email::Email;
use crate::{
    typosquat::{Cache, Crate},
    worker::Environment,
    Emails,
};

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

            let email = PossibleTyposquatEmail {
                domain: &emails.domain,
                crate_name: name,
                squats: &squats,
            };

            for recipient in cache.iter_emails() {
                if let Err(error) = emails.async_send(recipient, email.clone()).await {
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

#[derive(Debug, Clone)]
struct PossibleTyposquatEmail<'a> {
    domain: &'a str,
    crate_name: &'a str,
    squats: &'a [typomania::checks::Squat],
}

impl Email for PossibleTyposquatEmail<'_> {
    fn subject(&self) -> String {
        format!(
            "crates.io: Possible typosquatting in new crate \"{}\"",
            self.crate_name
        )
    }

    fn body(&self) -> String {
        let squats = self
            .squats
            .iter()
            .map(|squat| {
                let domain = self.domain;
                let crate_name = squat.package();
                format!("- {squat} (https://{domain}/crates/{crate_name})\n")
            })
            .collect::<Vec<_>>()
            .join("");

        format!(
            "New crate {crate_name} may be typosquatting one or more other crates.

Visit https://{domain}/crates/{crate_name} to see the offending crate.

Specific squat checks that triggered:

{squats}",
            domain = self.domain,
            crate_name = self.crate_name,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typosquat::test_util::faker;
    use crates_io_test_db::TestDatabase;
    use diesel_async::AsyncConnection;
    use lettre::Address;

    #[tokio::test]
    async fn integration() -> anyhow::Result<()> {
        let emails = Emails::new_in_memory();
        let test_db = TestDatabase::new();
        let mut conn = test_db.connect();

        // Set up a user and a popular crate to match against.
        let user = faker::user(&mut conn, "a")?;
        faker::crate_and_version(&mut conn, "my-crate", "It's awesome", &user, 100)?;

        // Prime the cache so it only includes the crate we just created.
        let mut async_conn = AsyncPgConnection::establish(test_db.url()).await?;
        let cache = Cache::new(vec!["admin@example.com".to_string()], &mut async_conn).await?;
        let cache = Arc::new(cache);

        // Now we'll create new crates: one problematic, one not so.
        let other_user = faker::user(&mut conn, "b")?;
        let angel = faker::crate_and_version(
            &mut conn,
            "innocent-crate",
            "I'm just a simple, innocent crate",
            &other_user,
            0,
        )?;
        let demon = faker::crate_and_version(
            &mut conn,
            "mycrate",
            "I'm even more innocent, obviously",
            &other_user,
            0,
        )?;

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
