use std::env;
use std::path::Path;

use crate::util::{bad_request, CargoResult};
use dotenv::dotenv;

use lettre::file::FileTransport;
use lettre::smtp::authentication::{Credentials, Mechanism};
use lettre::smtp::SmtpClient;
use lettre::{SendableEmail, Transport};

use lettre_email::Email;

#[derive(Debug)]
pub struct MailgunConfigVars {
    pub smtp_login: String,
    pub smtp_password: String,
    pub smtp_server: String,
}

pub fn init_config_vars() -> Option<MailgunConfigVars> {
    dotenv().ok();

    match (
        env::var("MAILGUN_SMTP_LOGIN"),
        env::var("MAILGUN_SMTP_PASSWORD"),
        env::var("MAILGUN_SMTP_SERVER"),
    ) {
        (Ok(login), Ok(password), Ok(server)) => Some(MailgunConfigVars {
            smtp_login: login,
            smtp_password: password,
            smtp_server: server,
        }),
        _ => None,
    }
}

fn build_email(
    recipient: &str,
    subject: &str,
    body: &str,
    mailgun_config: &Option<MailgunConfigVars>,
) -> CargoResult<SendableEmail> {
    let sender = mailgun_config
        .as_ref()
        .map(|s| s.smtp_login.as_str())
        .unwrap_or("test@localhost");

    let email = Email::builder()
        .to(recipient)
        .from(sender)
        .subject(subject)
        .body(body)
        .build()
        .map_err(|e| e.compat())?;

    Ok(email.into())
}

pub fn send_user_confirm_email(email: &str, user_name: &str, token: &str) -> CargoResult<()> {
    // Create a URL with token string as path to send to user
    // If user clicks on path, look email/user up in database,
    // make sure tokens match

    let subject = "Please confirm your email address";
    let body = format!(
        "Hello {}! Welcome to Crates.io. Please click the
link below to verify your email address. Thank you!\n
https://crates.io/confirm/{}",
        user_name, token
    );

    send_email(email, subject, &body)
}

fn send_email(recipient: &str, subject: &str, body: &str) -> CargoResult<()> {
    let mailgun_config = init_config_vars();
    let email = build_email(recipient, subject, body, &mailgun_config)?;

    match mailgun_config {
        Some(mailgun_config) => {
            let mut transport = SmtpClient::new_simple(&mailgun_config.smtp_server)?
                .credentials(Credentials::new(
                    mailgun_config.smtp_login,
                    mailgun_config.smtp_password,
                ))
                .smtp_utf8(true)
                .authentication_mechanism(Mechanism::Plain)
                .transport();

            let result = transport.send(email);
            result.map_err(|_| bad_request("Error in sending email"))?;
        }
        None => {
            let mut sender = FileTransport::new(Path::new("/tmp"));
            let result = sender.send(email);
            result.map_err(|_| bad_request("Email file could not be generated"))?;
        }
    }

    Ok(())
}
