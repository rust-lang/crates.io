use anyhow::Result;
use crates_io_github::{GitHubClient, RealGitHubClient};
use oauth2::AccessToken;
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

#[derive(clap::Parser, Debug)]
enum Request {
    CurrentUser {
        #[clap(long, env = "GITHUB_ACCESS_TOKEN", hide_env_values = true)]
        access_token: SecretString,
    },
    GetUser {
        name: String,
        #[clap(long, env = "GITHUB_ACCESS_TOKEN", hide_env_values = true)]
        access_token: SecretString,
    },
    OrgByName {
        org_name: String,
        #[clap(long, env = "GITHUB_ACCESS_TOKEN", hide_env_values = true)]
        access_token: SecretString,
    },
    TeamByName {
        org_name: String,
        team_name: String,
        #[clap(long, env = "GITHUB_ACCESS_TOKEN", hide_env_values = true)]
        access_token: SecretString,
    },
    OrgMembership {
        org_id: i32,
        username: String,
        #[clap(long, env = "GITHUB_ACCESS_TOKEN", hide_env_values = true)]
        access_token: SecretString,
    },
    TeamMembership {
        org_id: i32,
        team_id: i32,
        username: String,
        #[clap(long, env = "GITHUB_ACCESS_TOKEN", hide_env_values = true)]
        access_token: SecretString,
    },
    PublicKeys {
        client_id: String,
        #[clap(long, env = "GITHUB_CLIENT_SECRET", hide_env_values = true)]
        client_secret: SecretString,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    use clap::Parser;

    init_tracing();

    let client = Client::new();
    let github_client = RealGitHubClient::new(client);

    match Request::parse() {
        Request::CurrentUser { access_token } => {
            let access_token = AccessToken::new(access_token.expose_secret().into());
            let response = github_client.current_user(&access_token).await?;
            println!("{response:#?}");
        }
        Request::GetUser { name, access_token } => {
            let access_token = AccessToken::new(access_token.expose_secret().into());
            let response = github_client.get_user(&name, &access_token).await?;
            println!("{response:#?}");
        }
        Request::OrgByName {
            org_name,
            access_token,
        } => {
            let access_token = AccessToken::new(access_token.expose_secret().into());
            let response = github_client.org_by_name(&org_name, &access_token).await?;
            println!("{response:#?}");
        }
        Request::TeamByName {
            org_name,
            team_name,
            access_token,
        } => {
            let access_token = AccessToken::new(access_token.expose_secret().into());
            let response = github_client
                .team_by_name(&org_name, &team_name, &access_token)
                .await?;
            println!("{response:#?}");
        }
        Request::OrgMembership {
            org_id,
            username,
            access_token,
        } => {
            let access_token = AccessToken::new(access_token.expose_secret().into());
            let response = github_client
                .org_membership(org_id, &username, &access_token)
                .await?;
            println!("{response:#?}");
        }
        Request::TeamMembership {
            org_id,
            team_id,
            username,
            access_token,
        } => {
            let access_token = AccessToken::new(access_token.expose_secret().into());
            let response = github_client
                .team_membership(org_id, team_id, &username, &access_token)
                .await?;
            println!("{response:#?}");
        }
        Request::PublicKeys {
            client_id,
            client_secret,
        } => {
            let response = github_client
                .public_keys(&client_id, client_secret.expose_secret())
                .await?;
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
