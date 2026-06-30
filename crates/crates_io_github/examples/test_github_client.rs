use anyhow::Result;
use crates_io_github::{GitHubAuth, GitHubClient, RealGitHubClient};
use reqwest::Client;
use secrecy::SecretString;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

#[derive(clap::Parser, Debug)]
struct Options {
    #[clap(flatten)]
    auth: AuthArgs,

    #[clap(subcommand)]
    request: Request,
}

/// Authentication options shared by all requests.
///
/// When no credentials are provided the request is sent unauthenticated.
/// An access token results in bearer authentication, while the client
/// id/secret or username/password pairs result in HTTP basic authentication.
#[derive(clap::Args, Debug)]
struct AuthArgs {
    /// OAuth or personal access token used for bearer authentication.
    #[clap(long, env = "GITHUB_ACCESS_TOKEN", hide_env_values = true)]
    access_token: Option<SecretString>,

    /// OAuth client id used for basic authentication.
    #[clap(long, env = "GITHUB_CLIENT_ID", requires = "client_secret")]
    client_id: Option<String>,
    /// OAuth client secret used for basic authentication.
    #[clap(
        long,
        env = "GITHUB_CLIENT_SECRET",
        hide_env_values = true,
        requires = "client_id"
    )]
    client_secret: Option<SecretString>,

    /// Username used for basic authentication.
    #[clap(long, env = "GITHUB_USERNAME", requires = "password")]
    username: Option<String>,
    /// Password used for basic authentication.
    #[clap(
        long,
        env = "GITHUB_PASSWORD",
        hide_env_values = true,
        requires = "username"
    )]
    password: Option<SecretString>,
}

#[derive(clap::Subcommand, Debug)]
enum Request {
    CurrentUser,
    GetUser {
        name: String,
    },
    GetUserById {
        account_id: i64,
    },
    OrgByName {
        org_name: String,
    },
    TeamByName {
        org_name: String,
        team_name: String,
    },
    OrgMembership {
        org_id: i32,
        username: String,
    },
    TeamMembership {
        org_id: i32,
        team_id: i32,
        username: String,
    },
    PublicKeys,
}

impl AuthArgs {
    fn into_auth(self) -> GitHubAuth {
        if let Some(access_token) = self.access_token {
            GitHubAuth::bearer(access_token)
        } else if let (Some(username), Some(password)) = (self.client_id, self.client_secret) {
            GitHubAuth::basic(username, password)
        } else if let (Some(username), Some(password)) = (self.username, self.password) {
            GitHubAuth::basic(username, password)
        } else {
            GitHubAuth::None
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    use clap::Parser;

    init_tracing();

    let client = Client::new();
    let github_client = RealGitHubClient::new(client);

    let options = Options::parse();
    let auth = options.auth.into_auth();

    match options.request {
        Request::CurrentUser => {
            let response = github_client.current_user(&auth).await?;
            println!("{response:#?}");
        }
        Request::GetUser { name } => {
            let response = github_client.get_user(&name, &auth).await?;
            println!("{response:#?}");
        }
        Request::GetUserById { account_id } => {
            let response = github_client.get_user_by_id(account_id, &auth).await?;
            println!("{response:#?}");
        }
        Request::OrgByName { org_name } => {
            let response = github_client.org_by_name(&org_name, &auth).await?;
            println!("{response:#?}");
        }
        Request::TeamByName {
            org_name,
            team_name,
        } => {
            let response = github_client
                .team_by_name(&org_name, &team_name, &auth)
                .await?;
            println!("{response:#?}");
        }
        Request::OrgMembership { org_id, username } => {
            let response = github_client
                .org_membership(org_id, &username, &auth)
                .await?;
            println!("{response:#?}");
        }
        Request::TeamMembership {
            org_id,
            team_id,
            username,
        } => {
            let response = github_client
                .team_membership(org_id, team_id, &username, &auth)
                .await?;
            println!("{response:#?}");
        }
        Request::PublicKeys => {
            let response = github_client.public_keys(&auth).await?;
            println!("{response:#?}");
        }
    }

    Ok(())
}

fn init_tracing() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .from_env_lossy();

    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(env_filter)
        .init();
}
