use clap::Parser;
use crates_io_zulip::{RealZulipClient, ZulipClient};
use secrecy::SecretString;
use url::Url;

/// Sends a message to a Zulip channel using the crates.io Zulip client.
#[derive(Debug, Parser)]
struct Options {
    /// Base URL of the Zulip instance, if it differs from the bot email domain.
    #[arg(long, env = "ZULIP_BASE_URL")]
    base_url: Option<Url>,

    /// Email address of the Zulip bot.
    #[arg(long, env = "ZULIP_BOT_EMAIL")]
    bot_email: String,

    /// API key of the Zulip bot.
    #[arg(long, env = "ZULIP_API_KEY", hide_env_values = true)]
    api_key: SecretString,

    /// Name of the channel to send the message to.
    #[arg(long)]
    channel: String,

    /// Topic within the channel.
    #[arg(long)]
    topic: String,

    /// Message content in Zulip-flavored Markdown.
    content: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = Options::parse();

    let zulip = RealZulipClient::builder()
        .maybe_base_url(options.base_url)
        .bot_email(options.bot_email)
        .api_key(options.api_key)
        .build();

    zulip
        .send_channel_message(&options.channel, &options.topic, &options.content)
        .await?;

    Ok(())
}
