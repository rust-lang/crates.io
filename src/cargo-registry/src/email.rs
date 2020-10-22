use std::path::Path;

use crate::util::errors::{server_error, AppResult};

use lettre::transport::file::FileTransport;
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::transport::smtp::SmtpTransport;
use lettre::{Message, Transport};

#[derive(Debug)]
pub struct MailgunConfigVars {
    pub smtp_login: String,
    pub smtp_password: String,
    pub smtp_server: String,
}

pub fn init_config_vars() -> Option<MailgunConfigVars> {
    match (
        dotenv::var("MAILGUN_SMTP_LOGIN"),
        dotenv::var("MAILGUN_SMTP_PASSWORD"),
        dotenv::var("MAILGUN_SMTP_SERVER"),
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
) -> AppResult<Message> {
    let sender = mailgun_config
        .as_ref()
        .map(|s| s.smtp_login.as_str())
        .unwrap_or("test@localhost");

    let email = Message::builder()
        .to(recipient.parse()?)
        .from(sender.parse()?)
        .subject(subject)
        .body(body)?;

    Ok(email)
}

/// Attempts to send a confirmation email. Swallows all errors.
///
/// This function swallows any errors that occur while attempting to send the email. Some users
/// have an invalid email set in their GitHub profile, and we should let them sign in even though
/// we're trying to silently use their invalid address during signup and can't send them an email.
/// Use `try_send_user_confirm_email` when the user is directly trying to set their email.
pub fn send_user_confirm_email(email: &str, user_name: &str, token: &str) {
    let _ = try_send_user_confirm_email(email, user_name, token);
}

/// Attempts to send a confirmation email and returns errors.
///
/// For use in cases where we want to fail if an email is bad because the user is directly trying
/// to set their email correctly, as opposed to us silently trying to use the email from their
/// GitHub profile during signup.
pub fn try_send_user_confirm_email(email: &str, user_name: &str, token: &str) -> AppResult<()> {
    // Create a URL with token string as path to send to user
    // If user clicks on path, look email/user up in database,
    // make sure tokens match

    let subject = "Please confirm your email address";
    let body = format!(
        "Hello {}! Welcome to Crates.io. Please click the
link below to verify your email address. Thank you!\n
https://{}/confirm/{}",
        user_name,
        crate::config::domain_name(),
        token
    );

    send_email(email, subject, &body)
}

/// Attempts to send a crate owner invitation email. Swallows all errors.
///
/// Whether or not the email is sent, the invitation entry will be created in
/// the database and the user will see the invitation when they visit
/// https://crates.io/me/pending-invites/.
pub fn send_owner_invite_email(email: &str, user_name: &str, crate_name: &str, token: &str) {
    let subject = "Crate ownership invitation";
    let body = format!(
        "{} has invited you to become an owner of the crate {}!\n
Visit https://{domain}/accept-invite/{} to accept this invitation,
or go to https://{domain}/me/pending-invites to manage all of your crate ownership invitations.",
        user_name,
        crate_name,
        token,
        domain = crate::config::domain_name()
    );

    let _ = send_email(email, subject, &body);
}

fn send_email(recipient: &str, subject: &str, body: &str) -> AppResult<()> {
    let mailgun_config = init_config_vars();
    let email = build_email(recipient, subject, body, &mailgun_config)?;

    match mailgun_config {
        Some(mailgun_config) => {
            let transport = SmtpTransport::relay(&mailgun_config.smtp_server)
                .unwrap()
                .credentials(Credentials::new(
                    mailgun_config.smtp_login,
                    mailgun_config.smtp_password,
                ))
                .authentication(vec![Mechanism::Plain])
                .build();

            let result = transport.send(&email);
            result.map_err(|_| server_error("Error in sending email"))?;
        }
        None => {
            let sender = FileTransport::new(Path::new("/tmp"));
            let result = sender.send(&email);
            result.map_err(|_| server_error("Email file could not be generated"))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sending_to_invalid_email_fails() {
        let result = send_email(
            "String.Format(\"{0}.{1}@live.com\", FirstName, LastName)",
            "test",
            "test",
        );
        assert!(result.is_err());
    }

    #[test]
    fn sending_to_valid_email_succeeds() {
        let result = send_email("someone@example.com", "test", "test");
        assert!(result.is_ok());
    }
}
