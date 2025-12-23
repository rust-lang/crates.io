use crate::Env;
use crate::config;
use lettre::address::Envelope;
use lettre::message::Mailbox;
use lettre::message::header::ContentType;
use lettre::message::{MultiPart, SinglePart};
use lettre::transport::file::AsyncFileTransport;
use lettre::transport::smtp::AsyncSmtpTransport;
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::transport::stub::AsyncStubTransport;
use lettre::{Address, AsyncTransport, Message, Tokio1Executor};
use minijinja::Environment;
use rand::distr::{Alphanumeric, SampleString};
use serde::Serialize;
use std::sync::LazyLock;

static EMAIL_ENV: LazyLock<Environment<'static>> = LazyLock::new(|| {
    let mut env = Environment::new();

    // Load templates from the templates directory
    let entries = std::fs::read_dir("src/email/templates");
    let entries = entries.expect("Failed to read email templates directory");

    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");

        let path = entry.path();
        let file_type = entry.file_type().expect("Failed to get file type");

        // Handle base template files
        if file_type.is_file() && path.extension().and_then(|s| s.to_str()) == Some("j2") {
            let template_name = entry.file_name();
            let template_name = template_name.to_str();
            let template_name = template_name.expect("Invalid UTF-8 in template filename");

            let template_contents = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("Failed to read template {template_name}: {error}"));

            env.add_template_owned(template_name.to_string(), template_contents)
                .expect("Failed to add template");
        }

        if !file_type.is_dir() {
            continue;
        }

        // Handle email template directories
        let dir_name = entry.file_name();
        let email_name = dir_name.to_str();
        let email_name = email_name.expect("Invalid UTF-8 in email template directory name");

        // Load subject.txt.j2 file
        let subject_path = path.join("subject.txt.j2");
        let subject_contents = std::fs::read_to_string(&subject_path).unwrap_or_else(|error| {
            panic!("Failed to read subject template for {email_name}: {error}")
        });
        let filename = format!("{email_name}/subject.txt.j2");
        env.add_template_owned(filename, subject_contents)
            .expect("Failed to add subject template");

        // Load body.txt.j2 file
        let body_path = path.join("body.txt.j2");
        let body_contents = std::fs::read_to_string(&body_path).unwrap_or_else(|error| {
            panic!("Failed to read body template for {email_name}: {error}")
        });
        let filename = format!("{email_name}/body.txt.j2");
        env.add_template_owned(filename, body_contents)
            .expect("Failed to add body template");

        // Load body.html.j2 file
        let html_path = path.join("body.html.j2");
        let html_contents = std::fs::read_to_string(&html_path).unwrap_or_else(|error| {
            panic!("Failed to read HTML body template for {email_name}: {error}")
        });
        let filename = format!("{email_name}/body.html.j2");
        env.add_template_owned(filename, html_contents)
            .expect("Failed to add HTML body template");
    }

    env
});

fn render_template(
    template_name: &str,
    context: impl Serialize,
) -> Result<String, minijinja::Error> {
    EMAIL_ENV.get_template(template_name)?.render(context)
}

#[derive(Debug, Clone)]
pub struct EmailMessage {
    pub subject: String,
    pub body_text: String,
    pub body_html: String,
}

impl EmailMessage {
    pub fn from_template(
        template_name: &str,
        context: impl Serialize,
    ) -> Result<Self, minijinja::Error> {
        let subject = render_template(&format!("{template_name}/subject.txt.j2"), &context)?;
        let body_text = render_template(&format!("{template_name}/body.txt.j2"), &context)?;
        let body_html = render_template(&format!("{template_name}/body.html.j2"), &context)?;

        Ok(EmailMessage {
            subject,
            body_text,
            body_html,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Emails {
    backend: EmailBackend,
    pub domain: String,
    from: Address,
    html_emails_enabled: bool,
}

const DEFAULT_FROM: &str = "noreply@crates.io";

impl Emails {
    /// Create a new instance detecting the backend from the environment. This will either connect
    /// to a SMTP server or store the emails on the local filesystem.
    pub fn from_environment(config: &config::Server) -> Self {
        let login = dotenvy::var("MAILGUN_SMTP_LOGIN");
        let password = dotenvy::var("MAILGUN_SMTP_PASSWORD");
        let server = dotenvy::var("MAILGUN_SMTP_SERVER");

        let from = login.as_deref().unwrap_or(DEFAULT_FROM).parse().unwrap();

        let backend = match (login, password, server) {
            (Ok(login), Ok(password), Ok(server)) => {
                let transport = AsyncSmtpTransport::<Tokio1Executor>::relay(&server)
                    .unwrap()
                    .credentials(Credentials::new(login, password))
                    .authentication(vec![Mechanism::Plain])
                    .build();

                EmailBackend::Smtp(Box::new(transport))
            }
            _ => {
                let transport = AsyncFileTransport::new("/tmp");
                EmailBackend::FileSystem(transport)
            }
        };

        if config.base.env == Env::Production && !matches!(backend, EmailBackend::Smtp { .. }) {
            panic!("only the smtp backend is allowed in production");
        }

        let domain = config.domain_name.clone();

        let html_emails_enabled = dotenvy::var("HTML_EMAILS_ENABLED")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        Self {
            backend,
            domain,
            from,
            html_emails_enabled,
        }
    }

    /// Create a new test backend that stores all the outgoing emails in memory, allowing for tests
    /// to later assert the mails were sent.
    pub fn new_in_memory() -> Self {
        Self {
            backend: EmailBackend::Memory(AsyncStubTransport::new_ok()),
            domain: "crates.io".into(),
            from: DEFAULT_FROM.parse().unwrap(),
            html_emails_enabled: true,
        }
    }

    /// This is supposed to be used only during tests, to retrieve the messages stored in the
    /// "memory" backend. It's not cfg'd away because our integration tests need to access this.
    pub async fn mails_in_memory(&self) -> Option<Vec<(Envelope, String)>> {
        if let EmailBackend::Memory(transport) = &self.backend {
            Some(transport.messages().await)
        } else {
            None
        }
    }

    fn build_message(
        &self,
        recipient: &str,
        subject: String,
        body_text: String,
        body_html: String,
    ) -> Result<Message, EmailError> {
        // The message ID is normally generated by the SMTP server, but if we let it generate the
        // ID there will be no way for the crates.io application to know the ID of the message it
        // just sent, as it's not included in the SMTP response.
        //
        // Our support staff needs to know the message ID to be able to find misdelivered emails.
        // Because of that we're generating a random message ID, hoping the SMTP server doesn't
        // replace it when it relays the message.
        let message_id = format!(
            "<{}@{}>",
            Alphanumeric.sample_string(&mut rand::rng(), 32),
            self.domain,
        );

        let from = Mailbox::new(Some(self.domain.clone()), self.from.clone());

        let builder = Message::builder()
            .message_id(Some(message_id.clone()))
            .to(recipient.parse()?)
            .from(from)
            .subject(subject);

        let message = if self.html_emails_enabled {
            builder.multipart(
                MultiPart::alternative()
                    .singlepart(SinglePart::plain(body_text))
                    .singlepart(SinglePart::html(body_html)),
            )?
        } else {
            builder.header(ContentType::TEXT_PLAIN).body(body_text)?
        };

        Ok(message)
    }

    pub async fn send(&self, recipient: &str, email: EmailMessage) -> Result<(), EmailError> {
        let email =
            self.build_message(recipient, email.subject, email.body_text, email.body_html)?;

        self.backend
            .send(email)
            .await
            .map_err(EmailError::TransportError)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EmailError {
    #[error(transparent)]
    AddressError(#[from] lettre::address::AddressError),
    #[error(transparent)]
    MessageBuilderError(#[from] lettre::error::Error),
    #[error(transparent)]
    TransportError(anyhow::Error),
}

#[derive(Debug, Clone)]
enum EmailBackend {
    /// Backend used in production to send mails using SMTP.
    ///
    /// This is using `Box` to avoid a large size difference between variants.
    Smtp(Box<AsyncSmtpTransport<Tokio1Executor>>),
    /// Backend used locally during development, will store the emails in the provided directory.
    FileSystem(AsyncFileTransport<Tokio1Executor>),
    /// Backend used during tests, will keep messages in memory to allow tests to retrieve them.
    Memory(AsyncStubTransport),
}

impl EmailBackend {
    async fn send(&self, message: Message) -> anyhow::Result<()> {
        match self {
            EmailBackend::Smtp(transport) => transport.send(message).await.map(|_| ())?,
            EmailBackend::FileSystem(transport) => transport.send(message).await.map(|_| ())?,
            EmailBackend::Memory(transport) => transport.send(message).await.map(|_| ())?,
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
    use claims::{assert_err, assert_ok};
    use minijinja::context;

    #[test]
    fn test_user_confirm_template_inheritance() {
        // Test that the `user_confirm` template inherits properly from the base template
        let result = render_template(
            "user_confirm/body.txt.j2",
            context! {
                domain => "crates.io",
                user_name => "testuser",
                token => "abc123"
            },
        );
        assert_ok!(&result);

        let content = result.unwrap();
        insta::assert_snapshot!(content, @r"

        Hello testuser!

        Welcome to crates.io. Please click the link below to verify your email address:

        https://crates.io/confirm/abc123

        Thank you!

        --
        The crates.io Team
        ");
    }

    #[test]
    fn test_escaping() {
        let content = assert_ok!(render_template(
            "owner_invite/body.txt.j2",
            context! {
                inviter => "<script>alert('xss');</script>",
                domain => "crates.io",
                crate_name => "example-crate",
                token => "abc123"
            },
        ));

        insta::assert_snapshot!(content, @"

        <script>alert('xss');</script> has invited you to become an owner of the crate example-crate!

        Visit https://crates.io/accept-invite/abc123 to accept this invitation.

        You can also go to https://crates.io/me/pending-invites to manage all of your crate ownership invitations.

        --
        The crates.io Team
        ");

        let content = assert_ok!(render_template(
            "owner_invite/body.html.j2",
            context! {
                inviter => "<script>alert('xss');</script>",
                domain => "crates.io",
                crate_name => "example-crate",
                token => "abc123"
            },
        ));

        insta::assert_snapshot!(content, @r#"

        <p>&lt;script&gt;alert(&#x27;xss&#x27;);&lt;&#x2f;script&gt; has invited you to become an owner of the crate <strong>example-crate</strong>!</p>

        <p>Visit <a href="https://crates.io/accept-invite/abc123">https://crates.io/accept-invite/abc123</a> to accept this invitation.</p>

        <p>You can also go to <a href="https://crates.io/me/pending-invites">https://crates.io/me/pending-invites</a> to manage all of your crate ownership invitations.</p>

        <p>--<br>The crates.io Team</p>
        "#);
    }

    #[tokio::test]
    async fn sending_to_invalid_email_fails() {
        let emails = Emails::new_in_memory();

        let address = "String.Format(\"{0}.{1}@live.com\", FirstName, LastName)";
        let email = EmailMessage {
            subject: "test".into(),
            body_text: "test".into(),
            body_html: "<p>test</p>".into(),
        };
        assert_err!(emails.send(address, email).await);
    }

    #[tokio::test]
    async fn sending_to_valid_email_succeeds() {
        let emails = Emails::new_in_memory();

        let address = "someone@example.com";
        let email = EmailMessage {
            subject: "test".into(),
            body_text: "test".into(),
            body_html: "<p>test</p>".into(),
        };
        assert_ok!(emails.send(address, email).await);
    }
}
