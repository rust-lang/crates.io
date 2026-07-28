use anyhow::Context;
use chrono::{SecondsFormat, Utc};
use crates_io::config::Server;
use crates_io::email::{EmailMessage, Emails};
use minijinja::context;

#[derive(clap::Parser, Debug)]
#[command(
    name = "test-email",
    about = "Send a test publish notification email.",
    long_about = "Send a test publish notification email to the specified address. \
        This is useful for verifying that the email system is working correctly. \
        All template parameters can be customized, or defaults will be used."
)]
pub struct Opts {
    /// The email address to send the test email to
    #[arg(long, short)]
    email: String,

    /// The recipient name to use in the email greeting
    #[arg(long, default_value = "testuser")]
    recipient: String,

    /// The crate name to use in the email
    #[arg(long = "crate", default_value = "test-crate")]
    krate: String,

    /// The version number to use in the email
    #[arg(long, default_value = "1.0.0")]
    version: String,

    /// The publisher info string
    #[arg(long, default_value = " by testpublisher")]
    publisher_info: String,
}

pub async fn run(opts: Opts) -> anyhow::Result<()> {
    let config = Server::from_environment().context("Failed to load server configuration")?;
    let emails = Emails::from_environment(&config);

    let publish_time = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);

    let email = EmailMessage::from_template(
        "publish_notification",
        context! {
            recipient => opts.recipient,
            krate => opts.krate,
            version => opts.version,
            publish_time => publish_time,
            publisher_info => opts.publisher_info,
            domain => emails.domain
        },
    )
    .context("Failed to render email template")?;

    println!(
        "Sending test publish notification email to {}...",
        opts.email
    );
    println!();
    println!("Subject: {}", email.subject);
    println!();
    println!("Body:");
    println!("{}", email.body_text);

    emails
        .send(&opts.email, email)
        .await
        .context("Failed to send email")?;

    println!("Email sent successfully!");

    Ok(())
}
