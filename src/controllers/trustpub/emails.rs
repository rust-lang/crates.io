use crate::email::EmailMessage;
use crates_io_database::models::trustpub::GitHubConfig;
use crates_io_database::models::{Crate, User};

#[derive(serde::Serialize)]
pub struct ConfigCreatedEmail<'a> {
    /// The GitHub login of the email recipient.
    pub recipient: &'a str,
    /// The user who created the trusted publishing configuration.
    pub auth_user: &'a User,
    /// The crate for which the trusted publishing configuration was created.
    pub krate: &'a Crate,
    /// The trusted publishing configuration that was created.
    pub saved_config: &'a GitHubConfig,
}

impl ConfigCreatedEmail<'_> {
    pub fn render(&self) -> Result<EmailMessage, minijinja::Error> {
        EmailMessage::from_template("trustpub_config_created", self)
    }
}

#[derive(serde::Serialize)]
pub struct ConfigDeletedEmail<'a> {
    /// The GitHub login of the email recipient.
    pub recipient: &'a str,
    /// The user who deleted the trusted publishing configuration.
    pub auth_user: &'a User,
    /// The crate for which the trusted publishing configuration was deleted.
    pub krate: &'a Crate,
    /// The trusted publishing configuration that was deleted.
    pub config: &'a GitHubConfig,
}

impl ConfigDeletedEmail<'_> {
    pub fn render(&self) -> Result<EmailMessage, minijinja::Error> {
        EmailMessage::from_template("trustpub_config_deleted", self)
    }
}
