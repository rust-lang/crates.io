use dotenv::dotenv;
use std::env;
use std::path::Path;
use util::{CargoResult, bad_request};
use lettre::email::{EmailBuilder, Email};
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

fn init_config_vars() -> MailgunConfigVars {
    dotenv().ok();

    let mailgun_config = MailgunConfigVars {
        smtp_login: env::var("MAILGUN_SMTP_LOGIN").unwrap_or_else(|_| String::from("Not Found")),
        smtp_password: env::var("MAILGUN_SMTP_PASSWORD").unwrap_or_else(|_| {
            String::from("Not Found")
        }),
        smtp_server: env::var("MAILGUN_SMTP_SERVER").unwrap_or_else(|_| String::from("Not Found")),
    };

    mailgun_config
}

fn build_email(recipient: &str, subject: &str, body: &str, smtp_login: &str) -> Email {
    let email = EmailBuilder::new()
        .to(recipient)
        .from(smtp_login)
        .subject(subject)
        .body(body)
        .build()
        .expect("Failed to build confirm email message");

    email
}

pub fn send_email(recipient: &str, subject: &str, body: &str) -> CargoResult<()> {
    let mailgun_config = init_config_vars();
    let email = build_email(recipient, subject, body, &mailgun_config.smtp_login);

    if mailgun_config.smtp_login == "Not Found" && mailgun_config.smtp_password == "Not Found" &&
        mailgun_config.smtp_server == "Not Found"
    {
        let mut sender = FileEmailTransport::new(Path::new("/tmp"));
        let result = sender.send(email.clone());
        result.map_err(
            |_| bad_request("Email file could not be generated"),
        )?;
    } else {
        let mut transport = SmtpTransportBuilder::new(
            (mailgun_config.smtp_server.as_str(), SUBMISSION_PORT),
        ).expect("Failed to create message transport")
            .credentials(&mailgun_config.smtp_login, &mailgun_config.smtp_password)
            .security_level(SecurityLevel::AlwaysEncrypt)
            .smtp_utf8(true)
            .authentication_mechanism(Mechanism::Plain)
            .build();

        let result = transport.send(email.clone());
        result.map_err(|_| bad_request("Error in sending email"))?;
    }

    Ok(())
}
