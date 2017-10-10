use dotenv::dotenv;
use std::env;
use std::path::Path;
use util::{bad_request, CargoResult};
use lettre::email::{Email, EmailBuilder};
use lettre::transport::file::FileEmailTransport;
use lettre::transport::EmailTransport;
use lettre::transport::smtp::{SecurityLevel, SmtpTransportBuilder};
use lettre::transport::smtp::SUBMISSION_PORT;
use lettre::transport::smtp::authentication::Mechanism;

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

pub fn build_email(
    recipient: &str,
    subject: &str,
    body: &str,
    mailgun_config: &Option<MailgunConfigVars>,
) -> CargoResult<Email> {
    let sender = mailgun_config
        .as_ref()
        .map(|s| s.smtp_login.as_str())
        .unwrap_or("Development Mode");

    let email = EmailBuilder::new()
        .to(recipient)
        .from(sender)
        .subject(subject)
        .body(body)
        .build()?;

    Ok(email)
}

pub fn send_email(recipient: &str, subject: &str, body: &str) -> CargoResult<()> {
    let mailgun_config = init_config_vars();
    let email = build_email(recipient, subject, body, &mailgun_config)?;

    match mailgun_config {
        Some(mailgun_config) => {
            let mut transport =
                SmtpTransportBuilder::new((mailgun_config.smtp_server.as_str(), SUBMISSION_PORT))?
                    .credentials(&mailgun_config.smtp_login, &mailgun_config.smtp_password)
                    .security_level(SecurityLevel::AlwaysEncrypt)
                    .smtp_utf8(true)
                    .authentication_mechanism(Mechanism::Plain)
                    .build();

            let result = transport.send(email.clone());
            result.map_err(|_| bad_request("Error in sending email"))?;
        }
        None => {
            let mut sender = FileEmailTransport::new(Path::new("/tmp"));
            let result = sender.send(email.clone());
            result.map_err(|_| bad_request("Email file could not be generated"))?;
        }
    }

    Ok(())
}
