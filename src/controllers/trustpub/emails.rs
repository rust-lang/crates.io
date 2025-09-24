use crate::email::EmailMessage;
use crates_io_database::models::trustpub::GitHubConfig;
use crates_io_database::models::{Crate, User};

#[derive(serde::Serialize)]
pub struct ConfigCreatedEmail<'a> {
    pub recipient: &'a str,
    pub auth_user: &'a User,
    pub krate: &'a Crate,
    pub saved_config: &'a GitHubConfig,
}

impl ConfigCreatedEmail<'_> {
    pub fn render(&self) -> Result<EmailMessage, minijinja::Error> {
        EmailMessage::from_template("trustpub_config_created", self)
    }
}

#[derive(serde::Serialize)]
pub struct ConfigDeletedEmail<'a> {
    pub recipient: &'a str,
    pub auth_user: &'a User,
    pub krate: &'a Crate,
    pub config: &'a GitHubConfig,
}

impl ConfigDeletedEmail<'_> {
    pub fn render(&self) -> Result<EmailMessage, minijinja::Error> {
        EmailMessage::from_template("trustpub_config_deleted", self)
    }
}
