use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::util::errors::{server_error, BoxedAppError};

use crate::config;
use crate::Env;
use lettre::message::header::ContentType;
use lettre::transport::file::FileTransport;
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::transport::smtp::SmtpTransport;
use lettre::{Message, Transport};
use rand::distributions::{Alphanumeric, DistString};

#[derive(Debug, Clone)]
pub struct Emails {
    backend: EmailBackend,
}

impl Emails {
    /// Create a new instance detecting the backend from the environment. This will either connect
    /// to a SMTP server or store the emails on the local filesystem.
    pub fn from_environment(config: &config::Server) -> Self {
        let backend = match (
            dotenvy::var("MAILGUN_SMTP_LOGIN"),
            dotenvy::var("MAILGUN_SMTP_PASSWORD"),
            dotenvy::var("MAILGUN_SMTP_SERVER"),
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

        if config.base.env == Env::Production && !matches!(backend, EmailBackend::Smtp { .. }) {
            panic!("only the smtp backend is allowed in production");
        }

        Self { backend }
    }

    /// Create a new test backend that stores all the outgoing emails in memory, allowing for tests
    /// to later assert the mails were sent.
    pub fn new_in_memory() -> Self {
        Self {
            backend: EmailBackend::Memory {
                mails: Arc::new(Mutex::new(Vec::new())),
            },
        }
    }

    /// Attempts to send a confirmation email.
    pub fn send_user_confirm(
        &self,
        email: &str,
        user_name: &str,
        token: &str,
    ) -> Result<(), EmailError> {
        // Create a URL with token string as path to send to user
        // If user clicks on path, look email/user up in database,
        // make sure tokens match

        let subject = "Please confirm your email address";
        let body = format!(
            "Hello {}! Welcome to crates.io. Please click the
link below to verify your email address. Thank you!\n
https://{}/confirm/{}",
            user_name,
            crate::config::domain_name(),
            token
        );

        self.send(email, subject, &body)
    }

    /// Attempts to send an ownership invitation.
    pub fn send_owner_invite(
        &self,
        email: &str,
        user_name: &str,
        crate_name: &str,
        token: &str,
    ) -> Result<(), EmailError> {
        let subject = "Crate ownership invitation";
        let body = format!(
            "{user_name} has invited you to become an owner of the crate {crate_name}!\n
Visit https://{domain}/accept-invite/{token} to accept this invitation,
or go to https://{domain}/me/pending-invites to manage all of your crate ownership invitations.",
            domain = crate::config::domain_name()
        );

        self.send(email, subject, &body)
    }

    /// Attempts to send a notification that a new crate may be typosquatting another crate.
    pub fn send_possible_typosquat_notification(
        &self,
        email: &str,
        crate_name: &str,
        squats: &[typomania::checks::Squat],
    ) -> Result<(), EmailError> {
        let domain = crate::config::domain_name();
        let subject = "Possible typosquatting in new crate";
        let body = format!(
            "New crate {crate_name} may be typosquatting one or more other crates.\n
Visit https://{domain}/crates/{crate_name} to see the offending crate.\n
\n
Specific squat checks that triggered:\n
\n
{squats}",
            squats = squats
                .iter()
                .map(|squat| format!(
                    "- {squat} (https://{domain}/crates/{crate_name})\n",
                    crate_name = squat.package()
                ))
                .collect::<Vec<_>>()
                .join(""),
        );

        self.send(email, subject, &body)
    }

    /// Attempts to send an API token exposure notification email
    pub fn send_token_exposed_notification(
        &self,
        email: &str,
        url: &str,
        reporter: &str,
        source: &str,
        token_name: &str,
    ) -> Result<(), EmailError> {
        let subject = "Exposed API token found";
        let mut body = format!(
            "{reporter} has notified us that your crates.io API token {token_name}\n
has been exposed publicly. We have revoked this token as a precaution.\n
Please review your account at https://{domain} to confirm that no\n
unexpected changes have been made to your settings or crates.\n
\n
Source type: {source}\n",
            domain = crate::config::domain_name()
        );
        if url.is_empty() {
            body.push_str("\nWe were not informed of the URL where the token was found.\n");
        } else {
            body.push_str(&format!("\nURL where the token was found: {url}\n"));
        }
        self.send(email, subject, &body)
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

    fn send(&self, recipient: &str, subject: &str, body: &str) -> Result<(), EmailError> {
        // The message ID is normally generated by the SMTP server, but if we let it generate the
        // ID there will be no way for the crates.io application to know the ID of the message it
        // just sent, as it's not included in the SMTP response.
        //
        // Our support staff needs to know the message ID to be able to find misdelivered emails.
        // Because of that we're generating a random message ID, hoping the SMTP server doesn't
        // replace it when it relays the message.
        let message_id = format!(
            "<{}@{}>",
            Alphanumeric.sample_string(&mut rand::thread_rng(), 32),
            crate::config::domain_name(),
        );

        let email = Message::builder()
            .message_id(Some(message_id.clone()))
            .to(recipient.parse()?)
            .from(self.sender_address().parse()?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string())?;

        match &self.backend {
            EmailBackend::Smtp {
                server,
                login,
                password,
            } => {
                SmtpTransport::relay(server).and_then(|transport| {
                    transport
                        .credentials(Credentials::new(login.clone(), password.clone()))
                        .authentication(vec![Mechanism::Plain])
                        .build()
                        .send(&email)
                })?;

                info!(?message_id, ?subject, "Email sent");
            }
            EmailBackend::FileSystem { path } => {
                let id = FileTransport::new(path).send(&email)?;

                info!(
                    path = ?path.join(format!("{id}.eml")),
                    ?subject,
                    "Email sent"
                );
            }
            EmailBackend::Memory { mails } => {
                mails.lock().unwrap().push(StoredEmail {
                    to: recipient.into(),
                    subject: subject.into(),
                    body: body.into(),
                });
            }
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

#[derive(Debug, thiserror::Error)]
pub enum EmailError {
    #[error(transparent)]
    AddressError(#[from] lettre::address::AddressError),
    #[error(transparent)]
    MessageBuilderError(#[from] lettre::error::Error),
    #[error(transparent)]
    SmtpTransportError(#[from] lettre::transport::smtp::Error),
    #[error(transparent)]
    FileTransportError(#[from] lettre::transport::file::Error),
}

impl From<EmailError> for BoxedAppError {
    fn from(error: EmailError) -> Self {
        match error {
            EmailError::AddressError(error) => Box::new(error),
            EmailError::MessageBuilderError(error) => Box::new(error),
            EmailError::SmtpTransportError(error) => {
                error!(?error, "Failed to send email");
                server_error("Failed to send the email")
            }
            EmailError::FileTransportError(error) => {
                error!(?error, "Failed to send email");
                server_error("Email file could not be generated")
            }
        }
    }
}

#[derive(Clone)]
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
    Memory { mails: Arc<Mutex<Vec<StoredEmail>>> },
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
            "String.Format(\"{0}.{1}@live.com\", FirstName, LastName)",
            "test",
            "test",
        ));
    }

    #[test]
    fn sending_to_valid_email_succeeds() {
        let emails = Emails::new_in_memory();

        assert_ok!(emails.send("someone@example.com", "test", "test"));
    }
}
