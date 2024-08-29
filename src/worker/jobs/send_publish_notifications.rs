use crate::email::Email;
use crate::models::OwnerKind;
use crate::schema::{crate_owners, crates, emails, users, versions};
use crate::tasks::spawn_blocking;
use crate::worker::Environment;
use anyhow::anyhow;
use chrono::{NaiveDateTime, SecondsFormat};
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use std::sync::Arc;

/// Background job that sends email notifications to all crate owners when a
/// new crate version is published.
#[derive(Serialize, Deserialize)]
pub struct SendPublishNotificationsJob {
    version_id: i32,
}

impl SendPublishNotificationsJob {
    pub fn new(version_id: i32) -> Self {
        Self { version_id }
    }
}

impl BackgroundJob for SendPublishNotificationsJob {
    const JOB_NAME: &'static str = "send_publish_notifications";

    type Context = Arc<Environment>;

    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        let version_id = self.version_id;

        info!("Sending publish notifications for version {version_id}…");

        let mut conn = ctx.deadpool.get().await?;

        // Get crate name, version and other publish details
        let publish_details = PublishDetails::for_version(version_id, &mut conn).await?;

        let publish_time = publish_details
            .publish_time
            .and_utc()
            .to_rfc3339_opts(SecondsFormat::Secs, true);

        // Find names and email addresses of all crate owners
        let recipients = crate_owners::table
            .filter(crate_owners::deleted.eq(false))
            .filter(crate_owners::owner_kind.eq(OwnerKind::User))
            .filter(crate_owners::crate_id.eq(publish_details.crate_id))
            .inner_join(users::table)
            .inner_join(emails::table.on(users::id.eq(emails::user_id)))
            .filter(emails::verified.eq(true))
            .select((users::gh_login, emails::email))
            .load::<(String, String)>(&mut conn)
            .await?;

        let num_recipients = recipients.len();
        if num_recipients == 0 {
            info!(
                "Skipping publish notifications for {}@{}: no valid recipients found",
                publish_details.krate, publish_details.version
            );

            return Ok(());
        }

        // Sending emails is currently a blocking operation, so we have to use
        // `spawn_blocking()` to run it in a separate thread.
        spawn_blocking(move || {
            let results = recipients
                .into_iter()
                .map(|(ref recipient, email_address)| {
                    let krate = &publish_details.krate;
                    let version = &publish_details.version;

                    let publisher_info = match &publish_details.publisher {
                        Some(publisher) if publisher == recipient => &format!(
                            " by your account (https://{domain}/users/{publisher})",
                            domain = ctx.config.domain_name
                        ),
                        Some(publisher) => &format!(
                            " by {publisher} (https://{domain}/users/{publisher})",
                            domain = ctx.config.domain_name
                        ),
                        None => "",
                    };

                    let email = PublishNotificationEmail {
                        recipient,
                        krate,
                        version,
                        publish_time: &publish_time,
                        publisher_info,
                    };

                    debug!("Sending publish notification for {krate}@{version} to {email_address}…");
                    ctx.emails.send(&email_address, email).inspect_err(|err| {
                        warn!("Failed to send publish notification for {krate}@{version} to {email_address}: {err}")
                    })
                })
                .collect::<Vec<_>>();

            let num_sent = results.iter().filter(|result| result.is_ok()).count();

            // Check if *none* of the emails succeeded to send, in which case we
            // consider the job failed and worth retrying.
            if num_sent == 0 {
                warn!(
                    "Failed to send publish notifications for {}@{}",
                    publish_details.krate, publish_details.version
                );

                return Err(anyhow!("Failed to send publish notifications"));
            }

            if num_sent == num_recipients {
                info!(
                    "Sent {num_sent} publish notifications for {}@{}",
                    publish_details.krate, publish_details.version
                );
            } else {
                warn!(
                    "Sent only {num_sent} of {num_recipients} publish notifications for {}@{}",
                    publish_details.krate, publish_details.version
                );
            }

            Ok(())
        })
        .await
    }
}

#[derive(Debug, Queryable, Selectable)]
struct PublishDetails {
    #[diesel(select_expression = crates::columns::id)]
    crate_id: i32,
    #[diesel(select_expression = crates::columns::name)]
    krate: String,
    #[diesel(select_expression = versions::columns::num)]
    version: String,
    #[diesel(select_expression = versions::columns::created_at)]
    publish_time: NaiveDateTime,
    #[diesel(select_expression = users::columns::gh_login.nullable())]
    publisher: Option<String>,
}

impl PublishDetails {
    async fn for_version(version_id: i32, conn: &mut AsyncPgConnection) -> QueryResult<Self> {
        versions::table
            .find(version_id)
            .inner_join(crates::table)
            .left_join(users::table)
            .select(PublishDetails::as_select())
            .first(conn)
            .await
    }
}

/// Email template for notifying crate owners about a new crate version
/// being published.
#[derive(Debug, Clone)]
struct PublishNotificationEmail<'a> {
    recipient: &'a str,
    krate: &'a str,
    version: &'a str,
    publish_time: &'a str,
    publisher_info: &'a str,
}

impl Email for PublishNotificationEmail<'_> {
    fn subject(&self) -> String {
        let Self { krate, version, .. } = self;
        format!("crates.io: Successfully published {krate}@{version}")
    }

    fn body(&self) -> String {
        let Self {
            recipient,
            krate,
            version,
            publish_time,
            publisher_info,
        } = self;

        format!(
            "Hello {recipient}!

A new version of the package {krate} ({version}) was published{publisher_info} at {publish_time}.

If you have questions or security concerns, you can contact us at help@crates.io."
        )
    }
}
