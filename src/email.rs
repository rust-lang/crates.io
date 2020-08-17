use std::path::PathBuf;
use std::sync::Mutex;

use crate::util::errors::{server_error, AppResult};

use lettre::transport::file::FileTransport;
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::transport::smtp::SmtpTransport;
use lettre::{Message, Transport};

#[derive(Debug)]
pub struct Emails {
    backend: EmailBackend,
}

impl Emails {
    /// Create a new instance detecting the backend from the environment. This will either connect
    /// to a SMTP server or store the emails on the local filesystem.
    pub fn from_environment() -> Self {
        let backend = match (
            dotenv::var("MAILGUN_SMTP_LOGIN"),
            dotenv::var("MAILGUN_SMTP_PASSWORD"),
            dotenv::var("MAILGUN_SMTP_SERVER"),
        ) {
            (Ok(login), Ok(password), Ok(server)) => EmailBackend::Smtp {
                server,
                login,
                password,
            },
            _ => EmailBackend::FileSystem {
                path: "/tmp".into(),
            },
        };

        Self { backend }
    }

    /// Create a new test backend that stores all the outgoing emails in memory, allowing for tests
    /// to later assert the mails were sent.
    pub fn new_in_memory() -> Self {
        Self {
            backend: EmailBackend::Memory {
                mails: Mutex::new(Vec::new()),
            },
        }
    }

    /// Attempts to send a confirmation email.
    pub fn send_user_confirm(&self, email: &str, user_name: &str, token: &str) -> AppResult<()> {
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

        self.send(&[email], subject, &body)
    }

    /// Attempts to send an ownership invitation.
    pub fn send_owner_invite(
        &self,
        email: &str,
        user_name: &str,
        crate_name: &str,
        token: &str,
    ) -> AppResult<()> {
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

        self.send(&[email], subject, &body)
    }

    /// Attempts to send a new crate version published notification to crate owners.
    pub fn notify_owners(
        &self,
        emails: &[&str],
        crate_name: &str,
        crate_version: &str,
        publisher_name: Option<&str>,
        publisher_email: &str,
    ) -> AppResult<()> {
        let subject = format!(
            "Crate {} ({}) published to {}",
            crate_name,
            crate_version,
            crate::config::domain_name()
        );
        let body = format!(
            "A crate you have publish access to has recently released a new version.
Crate: {} ({})
Published by: {} <{}>
Published at: {}
If this publish is expected, you do not need to take further action.
Only if this publish is unexpected, please take immediate steps to secure your account:
* If you suspect your GitHub account was compromised, change your password
* Revoke your API Token
* Yank the version of the crate reported in this email
* Report this incident to RustSec https://rustsec.org
To stop receiving these messages, update your email notification settings at https://{domain}/me",
            crate_name,
            crate_version,
            publisher_name.unwrap_or("(unknown username)"),
            publisher_email,
            chrono::Utc::now().to_rfc2822(),
            domain = crate::config::domain_name()
        );

        self.send(emails, &subject, &body)
    }

    /// This is supposed to be used only during tests, to retrieve the messages stored in the
    /// "memory" backend. It's not cfg'd away because our integration tests need to access this.
    pub fn mails_in_memory(&self) -> Option<Vec<StoredEmail>> {
        if let EmailBackend::Memory { mails } = &self.backend {
            Some(mails.lock().unwrap().clone())
        } else {
            None
        }
    }

    fn send(&self, recipients: &[&str], subject: &str, body: &str) -> AppResult<()> {
        let mut recipients_iter = recipients.iter();

        let mut builder = Message::builder();
        let to = recipients_iter
            .next()
            .ok_or_else(|| server_error("Email has no recipients"))?;
        builder = builder.to(to.parse()?);
        for bcc in recipients_iter {
            builder = builder.bcc(bcc.parse()?);
        }

        let email = builder
            .from(self.sender_address().parse()?)
            .subject(subject)
            .body(body.to_string())?;

        match &self.backend {
            EmailBackend::Smtp {
                server,
                login,
                password,
            } => {
                SmtpTransport::relay(&server)?
                    .credentials(Credentials::new(login.clone(), password.clone()))
                    .authentication(vec![Mechanism::Plain])
                    .build()
                    .send(&email)
                    .map_err(|_| server_error("Error in sending email"))?;
            }
            EmailBackend::FileSystem { path } => {
                FileTransport::new(&path)
                    .send(&email)
                    .map_err(|_| server_error("Email file could not be generated"))?;
            }
            EmailBackend::Memory { mails } => mails.lock().unwrap().push(StoredEmail {
                to: to.to_string(),
                bcc: recipients
                    .iter()
                    .skip(1)
                    .map(|address| address.to_string())
                    .collect(),
                subject: subject.into(),
                body: body.into(),
            }),
        }

        Ok(())
    }

    fn sender_address(&self) -> &str {
        match &self.backend {
            EmailBackend::Smtp { login, .. } => login,
            EmailBackend::FileSystem { .. } => "test@localhost",
            EmailBackend::Memory { .. } => "test@localhost",
        }
    }
}

enum EmailBackend {
    /// Backend used in production to send mails using SMTP.
    Smtp {
        server: String,
        login: String,
        password: String,
    },
    /// Backend used locally during development, will store the emails in the provided directory.
    FileSystem { path: PathBuf },
    /// Backend used during tests, will keep messages in memory to allow tests to retrieve them.
    Memory { mails: Mutex<Vec<StoredEmail>> },
}

// Custom Debug implementation to avoid showing the SMTP password.
impl std::fmt::Debug for EmailBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmailBackend::Smtp { server, login, .. } => {
                // The password field is *intentionally* not included
                f.debug_struct("Smtp")
                    .field("server", server)
                    .field("login", login)
                    .finish()?;
            }
            EmailBackend::FileSystem { path } => {
                f.debug_struct("FileSystem").field("path", path).finish()?;
            }
            EmailBackend::Memory { .. } => f.write_str("Memory")?,
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct StoredEmail {
    pub to: String,
    pub bcc: Vec<String>,
    pub subject: String,
    pub body: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sending_to_invalid_email_fails() {
        let emails = Emails::new_in_memory();

        assert_err!(emails.send(
            &["String.Format(\"{0}.{1}@live.com\", FirstName, LastName)"],
            "test",
            "test",
        ));
    }

    #[test]
    fn sending_to_valid_email_succeeds() {
        let emails = Emails::new_in_memory();

        assert_ok!(emails.send(&["someone@example.com"], "test", "test"));
    }
}
