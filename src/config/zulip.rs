use crates_io_env_vars::{required_var, var};
use secrecy::SecretString;

/// Configuration for the Zulip bot that posts operational notices.
#[derive(Debug)]
pub struct ZulipConfig {
    /// Email address of the Zulip bot.
    ///
    /// Read from the `ZULIP_BOT_EMAIL` environment variable.
    pub bot_email: String,

    /// API key of the Zulip bot.
    ///
    /// Read from the `ZULIP_API_KEY` environment variable.
    pub api_key: SecretString,
}

impl ZulipConfig {
    /// Loads the optional Zulip configuration from environment variables.
    pub fn from_env() -> anyhow::Result<Option<Self>> {
        let Some(api_key) = var("ZULIP_API_KEY")? else {
            return Ok(None);
        };
        let bot_email = required_var("ZULIP_BOT_EMAIL")?;

        Ok(Some(Self {
            bot_email,
            api_key: api_key.into(),
        }))
    }
}
