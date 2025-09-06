use crate::email::EmailMessage;
use crate::models::{OwnerKind, TrustpubData};
use crate::schema::{crate_owners, crates, emails, users, versions};
use crate::worker::Environment;
use anyhow::anyhow;
use chrono::{DateTime, SecondsFormat, Utc};
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use minijinja::context;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

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
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        let version_id = self.version_id;

        info!("Sending publish notifications for version {version_id}…");

        let mut conn = ctx.deadpool.get().await?;

        // Get crate name, version and other publish details
        let Some(publish_details) = PublishDetails::for_version(version_id, &mut conn).await?
        else {
            warn!("Skipping publish notifications for {version_id}: no version found");

            return Ok(());
        };

        let publish_time = publish_details
            .publish_time
            .to_rfc3339_opts(SecondsFormat::Secs, true);

        // Find names and email addresses of all crate owners
        let recipients = crate_owners::table
            .filter(crate_owners::deleted.eq(false))
            .filter(crate_owners::owner_kind.eq(OwnerKind::User))
            .filter(crate_owners::crate_id.eq(publish_details.crate_id))
            .inner_join(users::table)
            .filter(users::publish_notifications.eq(true))
            .inner_join(emails::table.on(users::id.eq(emails::user_id)))
            .filter(emails::verified.eq(true))
            .filter(emails::primary.eq(true))
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

        let mut results = Vec::with_capacity(recipients.len());

        for (ref recipient, email_address) in recipients {
            let krate = &publish_details.krate;
            let version = &publish_details.version;

            let publisher_info = match (&publish_details.publisher, &publish_details.trustpub_data)
            {
                (Some(publisher), _) if publisher == recipient => &format!(
                    " by your account (https://{domain}/users/{publisher})",
                    domain = ctx.config.domain_name
                ),
                (Some(publisher), _) => &format!(
                    " by {publisher} (https://{domain}/users/{publisher})",
                    domain = ctx.config.domain_name
                ),
                (
                    _,
                    Some(TrustpubData::GitHub {
                        repository, run_id, ..
                    }),
                ) => &format!(
                    " by GitHub Actions (https://github.com/{repository}/actions/runs/{run_id})",
                ),
                _ => "",
            };

            let email = EmailMessage::from_template(
                "publish_notification",
                context! {
                    recipient => recipient,
                    krate => krate,
                    version => version,
                    publish_time => publish_time,
                    publisher_info => publisher_info
                },
            );

            debug!("Sending publish notification for {krate}@{version} to {email_address}…");
            let result = match email {
                Ok(email_msg) => {
                    ctx.emails.send(&email_address, email_msg).await.inspect_err(|err| {
                        warn!("Failed to send publish notification for {krate}@{version} to {email_address}: {err}")
                    }).map_err(|_| ())
                }
                Err(err) => {
                    warn!("Failed to render publish notification email template for {krate}@{version} to {email_address}: {err}");
                    Err(())
                }
            };

            results.push(result);
        }

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
    publish_time: DateTime<Utc>,
    #[diesel(select_expression = users::columns::gh_login.nullable())]
    publisher: Option<String>,
    #[diesel(select_expression = versions::columns::trustpub_data.nullable())]
    trustpub_data: Option<TrustpubData>,
}

impl PublishDetails {
    async fn for_version(
        version_id: i32,
        conn: &mut AsyncPgConnection,
    ) -> QueryResult<Option<Self>> {
        versions::table
            .find(version_id)
            .inner_join(crates::table)
            .left_join(users::table)
            .select(PublishDetails::as_select())
            .first(conn)
            .await
            .optional()
    }
}
