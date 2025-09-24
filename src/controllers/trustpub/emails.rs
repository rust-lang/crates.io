use crate::email::EmailMessage;
use crates_io_database::models::trustpub::{GitHubConfig, GitLabConfig};
use crates_io_database::models::{Crate, User};

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(tag = "type")]
pub enum ConfigType<'a> {
    GitHub(&'a GitHubConfig),
    GitLab(&'a GitLabConfig),
}

#[derive(serde::Serialize)]
pub struct ConfigCreatedEmail<'a> {
    /// The GitHub login of the email recipient.
    pub recipient: &'a str,
    /// The user who created the trusted publishing configuration.
    pub auth_user: &'a User,
    /// The crate for which the trusted publishing configuration was created.
    pub krate: &'a Crate,
    /// The trusted publishing configuration that was created.
    pub saved_config: ConfigType<'a>,
}

impl ConfigCreatedEmail<'_> {
    pub fn render(&self) -> Result<EmailMessage, minijinja::Error> {
        EmailMessage::from_template("trustpub_config_created", self)
    }
}

#[derive(serde::Serialize)]
pub struct ConfigDeletedEmail<'a> {
    /// The GitHub login of the email recipient.
    pub recipient: &'a str,
    /// The user who deleted the trusted publishing configuration.
    pub auth_user: &'a User,
    /// The crate for which the trusted publishing configuration was deleted.
    pub krate: &'a Crate,
    /// The trusted publishing configuration that was deleted.
    pub config: ConfigType<'a>,
}

impl ConfigDeletedEmail<'_> {
    pub fn render(&self) -> Result<EmailMessage, minijinja::Error> {
        EmailMessage::from_template("trustpub_config_deleted", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use claims::assert_ok;
    use insta::assert_snapshot;

    fn test_user() -> User {
        User {
            id: 1,
            gh_login: "octocat".into(),
            name: Some("The Octocat".into()),
            gh_id: 123,
            gh_avatar: None,
            gh_encrypted_token: vec![],
            account_lock_reason: None,
            account_lock_until: None,
            is_admin: false,
            publish_notifications: true,
        }
    }

    fn test_crate() -> Crate {
        Crate {
            id: 1,
            name: "my-crate".into(),
            updated_at: Utc::now(),
            created_at: Utc::now(),
            description: None,
            homepage: None,
            documentation: None,
            repository: None,
            max_upload_size: None,
            max_features: None,
        }
    }

    fn test_github_config(environment: Option<&str>) -> GitHubConfig {
        GitHubConfig {
            id: 1,
            created_at: Utc::now(),
            crate_id: 1,
            repository_owner_id: 42,
            repository_owner: "rust-lang".into(),
            repository_name: "rust".into(),
            workflow_filename: "publish.yml".into(),
            environment: environment.map(String::from),
        }
    }

    fn test_gitlab_config(environment: Option<&str>) -> GitLabConfig {
        GitLabConfig {
            id: 1,
            created_at: Utc::now(),
            crate_id: 1,
            namespace_id: None,
            namespace: "rust-lang".into(),
            project: "my-crate".into(),
            workflow_filepath: ".gitlab-ci.yml".into(),
            environment: environment.map(String::from),
        }
    }

    #[test]
    fn test_config_created_email() {
        let email = ConfigCreatedEmail {
            recipient: "octocat",
            auth_user: &test_user(),
            krate: &test_crate(),
            saved_config: ConfigType::GitHub(&test_github_config(None)),
        };

        let rendered = assert_ok!(email.render());
        assert_snapshot!(rendered.subject, @"crates.io: Trusted Publishing configuration added to my-crate");
        assert_snapshot!(rendered.body_text);
    }

    #[test]
    fn test_config_created_email_with_environment() {
        let email = ConfigCreatedEmail {
            recipient: "octocat",
            auth_user: &test_user(),
            krate: &test_crate(),
            saved_config: ConfigType::GitHub(&test_github_config(Some("production"))),
        };

        let rendered = assert_ok!(email.render());
        assert_snapshot!(rendered.subject, @"crates.io: Trusted Publishing configuration added to my-crate");
        assert_snapshot!(rendered.body_text);
    }

    #[test]
    fn test_config_created_email_different_recipient() {
        let email = ConfigCreatedEmail {
            recipient: "team-member",
            auth_user: &test_user(),
            krate: &test_crate(),
            saved_config: ConfigType::GitHub(&test_github_config(None)),
        };

        let rendered = assert_ok!(email.render());
        assert_snapshot!(rendered.subject, @"crates.io: Trusted Publishing configuration added to my-crate");
        assert_snapshot!(rendered.body_text);
    }

    #[test]
    fn test_config_created_email_gitlab() {
        let email = ConfigCreatedEmail {
            recipient: "octocat",
            auth_user: &test_user(),
            krate: &test_crate(),
            saved_config: ConfigType::GitLab(&test_gitlab_config(None)),
        };

        let rendered = assert_ok!(email.render());
        assert_snapshot!(rendered.subject, @"crates.io: Trusted Publishing configuration added to my-crate");
        assert_snapshot!(rendered.body_text);
    }

    #[test]
    fn test_config_created_email_gitlab_with_environment() {
        let email = ConfigCreatedEmail {
            recipient: "octocat",
            auth_user: &test_user(),
            krate: &test_crate(),
            saved_config: ConfigType::GitLab(&test_gitlab_config(Some("production"))),
        };

        let rendered = assert_ok!(email.render());
        assert_snapshot!(rendered.subject, @"crates.io: Trusted Publishing configuration added to my-crate");
        assert_snapshot!(rendered.body_text);
    }

    #[test]
    fn test_config_deleted_email() {
        let email = ConfigDeletedEmail {
            recipient: "octocat",
            auth_user: &test_user(),
            krate: &test_crate(),
            config: ConfigType::GitHub(&test_github_config(None)),
        };

        let rendered = assert_ok!(email.render());
        assert_snapshot!(rendered.subject, @"crates.io: Trusted Publishing configuration removed from my-crate");
        assert_snapshot!(rendered.body_text);
    }

    #[test]
    fn test_config_deleted_email_with_environment() {
        let email = ConfigDeletedEmail {
            recipient: "octocat",
            auth_user: &test_user(),
            krate: &test_crate(),
            config: ConfigType::GitHub(&test_github_config(Some("production"))),
        };

        let rendered = assert_ok!(email.render());
        assert_snapshot!(rendered.subject, @"crates.io: Trusted Publishing configuration removed from my-crate");
        assert_snapshot!(rendered.body_text);
    }

    #[test]
    fn test_config_deleted_email_different_recipient() {
        let email = ConfigDeletedEmail {
            recipient: "team-member",
            auth_user: &test_user(),
            krate: &test_crate(),
            config: ConfigType::GitHub(&test_github_config(None)),
        };

        let rendered = assert_ok!(email.render());
        assert_snapshot!(rendered.subject, @"crates.io: Trusted Publishing configuration removed from my-crate");
        assert_snapshot!(rendered.body_text);
    }

    #[test]
    fn test_config_deleted_email_gitlab() {
        let email = ConfigDeletedEmail {
            recipient: "octocat",
            auth_user: &test_user(),
            krate: &test_crate(),
            config: ConfigType::GitLab(&test_gitlab_config(None)),
        };

        let rendered = assert_ok!(email.render());
        assert_snapshot!(rendered.subject, @"crates.io: Trusted Publishing configuration removed from my-crate");
        assert_snapshot!(rendered.body_text);
    }

    #[test]
    fn test_config_deleted_email_gitlab_with_environment() {
        let email = ConfigDeletedEmail {
            recipient: "octocat",
            auth_user: &test_user(),
            krate: &test_crate(),
            config: ConfigType::GitLab(&test_gitlab_config(Some("production"))),
        };

        let rendered = assert_ok!(email.render());
        assert_snapshot!(rendered.subject, @"crates.io: Trusted Publishing configuration removed from my-crate");
        assert_snapshot!(rendered.body_text);
    }
}
