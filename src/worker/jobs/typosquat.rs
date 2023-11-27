use async_trait::async_trait;
use std::sync::Arc;

use crates_io_worker::BackgroundJob;
use diesel::PgConnection;
use typomania::Package;

use crate::tasks::spawn_blocking;
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

#[async_trait]
impl BackgroundJob for CheckTyposquat {
    const JOB_NAME: &'static str = "check_typosquat";

    type Context = Arc<Environment>;

    #[instrument(skip(env), err)]
    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        let crate_name = self.name.clone();

        spawn_blocking(move || {
            let mut conn = env.connection_pool.get()?;
            let cache = env.typosquat_cache(&mut conn)?;
            check(&env.emails, cache, &mut conn, &crate_name)
        })
        .await
    }
}

fn check(
    emails: &Emails,
    cache: &Cache,
    conn: &mut PgConnection,
    name: &str,
) -> anyhow::Result<()> {
    if let Some(harness) = cache.get_harness() {
        info!(name, "Checking new crate for potential typosquatting");

        let krate: Box<dyn Package> = Box::new(Crate::from_name(conn, name)?);
        let squats = harness.check_package(name, krate)?;
        if !squats.is_empty() {
            // Well, well, well. For now, the only action we'll take is to e-mail people who
            // hopefully care to check into things more closely.
            info!(?squats, "Found potential typosquatting");

            for email in cache.iter_emails() {
                if let Err(e) = emails.send_possible_typosquat_notification(email, name, &squats) {
                    error!(?e, ?email, "Failed to send possible typosquat notification");
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{test_util::pg_connection, typosquat::test_util::Faker};

    use super::*;

    #[test]
    fn integration() -> anyhow::Result<()> {
        let emails = Emails::new_in_memory();
        let mut faker = Faker::new(pg_connection());

        // Set up a user and a popular crate to match against.
        let user = faker.user("a")?;
        faker.crate_and_version("my-crate", "It's awesome", &user, 100)?;

        // Prime the cache so it only includes the crate we just created.
        let cache = Cache::new(vec!["admin@example.com".to_string()], faker.borrow_conn())?;

        // Now we'll create new crates: one problematic, one not so.
        let other_user = faker.user("b")?;
        let (angel, _version) = faker.crate_and_version(
            "innocent-crate",
            "I'm just a simple, innocent crate",
            &other_user,
            0,
        )?;
        let (demon, _version) = faker.crate_and_version(
            "mycrate",
            "I'm even more innocent, obviously",
            &other_user,
            0,
        )?;

        // OK, we're done faking stuff.
        let mut conn = faker.into_conn();

        // Run the check with a crate that shouldn't cause problems.
        check(&emails, &cache, &mut conn, &angel.name)?;
        assert!(emails.mails_in_memory().unwrap().is_empty());

        // Now run the check with a less innocent crate.
        check(&emails, &cache, &mut conn, &demon.name)?;
        let sent_mail = emails.mails_in_memory().unwrap();
        assert!(!sent_mail.is_empty());
        let sent = sent_mail.into_iter().next().unwrap();
        assert_eq!(&sent.to, "admin@example.com");

        Ok(())
    }
}
