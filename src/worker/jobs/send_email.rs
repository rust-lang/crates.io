use crate::email::EmailMessage;
use crate::worker::Environment;
use crates_io_worker::BackgroundJob;
use lettre::Address;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Delivers a rendered email to one recipient.
#[derive(Serialize, Deserialize)]
pub struct SendEmail {
    recipient: Address,
    email: EmailMessage,
}

impl SendEmail {
    /// Prepares a delivery job without sending the email.
    pub fn new(recipient: Address, email: EmailMessage) -> Self {
        Self { recipient, email }
    }
}

impl BackgroundJob for SendEmail {
    const JOB_NAME: &'static str = "send_email";

    type Context = Arc<Environment>;

    async fn run(self, ctx: Self::Context) -> anyhow::Result<()> {
        let recipient = self.recipient.as_ref();
        ctx.emails.send(recipient, self.email).await?;
        Ok(())
    }
}
