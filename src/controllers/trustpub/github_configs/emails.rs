use crate::email::Email;

/// Email template for notifying crate owners about a new crate version
/// being published.
#[derive(Debug, Clone)]
pub struct ConfigCreatedEmail<'a> {
    pub recipient: &'a str,
    pub user: &'a str,
    pub krate: &'a str,
    pub repository_owner: &'a str,
    pub repository_name: &'a str,
    pub workflow_filename: &'a str,
    pub environment: &'a str,
}

impl Email for ConfigCreatedEmail<'_> {
    fn subject(&self) -> String {
        let Self { krate, .. } = self;
        format!("crates.io: Trusted Publishing configration added to {krate}")
    }

    fn body(&self) -> String {
        let Self {
            recipient,
            user,
            krate,
            repository_owner,
            repository_name,
            workflow_filename,
            environment,
        } = self;

        format!(
            "Hello {recipient}!

crates.io user {user} has added a new \"Trusted Publishing\" configuration for GitHub Actions to a crate that you manage ({krate}). Trusted publishers act as trusted users and can publish new versions of the crate automatically.

Trusted Publishing configuration:

- Repository owner: {repository_owner}
- Repository name: {repository_name}
- Workflow filename: {workflow_filename}
- Environment: {environment}

If you did not make this change and you think it was made maliciously, you can remove the configuration from the crate via the \"Settings\" tab on the crate's page.

If you are unable to revert the change and need to do so, you can email help@crates.io to communicate with the crates.io support team."
        )
    }
}

/// Email template for notifying crate owners about a Trusted Publishing
/// configuration being deleted.
#[derive(Debug, Clone)]
pub struct ConfigDeletedEmail<'a> {
    pub recipient: &'a str,
    pub user: &'a str,
    pub krate: &'a str,
    pub repository_owner: &'a str,
    pub repository_name: &'a str,
    pub workflow_filename: &'a str,
    pub environment: &'a str,
}

impl Email for ConfigDeletedEmail<'_> {
    fn subject(&self) -> String {
        let Self { krate, .. } = self;
        format!("crates.io: Trusted Publishing configration removed from {krate}")
    }

    fn body(&self) -> String {
        let Self {
            recipient,
            user,
            krate,
            repository_owner,
            repository_name,
            workflow_filename,
            environment,
        } = self;

        format!(
            "Hello {recipient}!

crates.io user {user} has remove a \"Trusted Publishing\" configuration for GitHub Actions from a crate that you manage ({krate}).

Trusted Publishing configuration:

- Repository owner: {repository_owner}
- Repository name: {repository_name}
- Workflow filename: {workflow_filename}
- Environment: {environment}

If you did not make this change and you think it was made maliciously, you can email help@crates.io to communicate with the crates.io support team."
        )
    }
}
