#![doc = include_str!("../README.md")]

#[macro_use]
extern crate tracing;

use oauth2::AccessToken;
use reqwest::{self, RequestBuilder, header};

use serde::de::DeserializeOwned;

use std::str;

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

type Result<T> = std::result::Result<T, GitHubError>;

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait GitHubClient: Send + Sync {
    async fn current_user(&self, auth: &AccessToken) -> Result<GitHubUser>;
    async fn get_user(&self, name: &str, auth: &AccessToken) -> Result<GitHubUser>;
    async fn org_by_name(&self, org_name: &str, auth: &AccessToken) -> Result<GitHubOrganization>;
    async fn team_by_name(
        &self,
        org_name: &str,
        team_name: &str,
        auth: &AccessToken,
    ) -> Result<GitHubTeam>;
    async fn team_membership(
        &self,
        org_id: i32,
        team_id: i32,
        username: &str,
        auth: &AccessToken,
    ) -> Result<Option<GitHubTeamMembership>>;
    async fn org_membership(
        &self,
        org_id: i32,
        username: &str,
        auth: &AccessToken,
    ) -> Result<Option<GitHubOrgMembership>>;
    async fn public_keys(&self, username: &str, password: &str) -> Result<Vec<GitHubPublicKey>>;
}

#[derive(Debug)]
pub struct RealGitHubClient {
    client: Client,
}

impl RealGitHubClient {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Does all the nonsense for sending a GET to GitHub.
    async fn _request<T, A>(&self, url: &str, apply_auth: A) -> Result<T>
    where
        T: DeserializeOwned,
        A: Fn(RequestBuilder) -> RequestBuilder,
    {
        let url = format!("https://api.github.com{url}");
        info!("GitHub request: GET {url}");

        let request = self
            .client
            .get(&url)
            .header(header::ACCEPT, "application/vnd.github.v3+json")
            .header(header::USER_AGENT, "crates.io (https://crates.io)");

        let response = apply_auth(request).send().await?.error_for_status()?;

        let headers = response.headers();
        let remaining = headers.get("x-ratelimit-remaining");
        let limit = headers.get("x-ratelimit-limit");
        debug!("GitHub rate limit remaining: {remaining:?}/{limit:?}");

        response.json().await.map_err(Into::into)
    }

    /// Sends a GET to GitHub using OAuth access token authentication
    pub async fn request<T>(&self, url: &str, auth: &AccessToken) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self._request(url, |r| r.bearer_auth(auth.secret())).await
    }

    /// Sends a GET to GitHub using basic authentication
    pub async fn request_basic<T>(&self, url: &str, username: &str, password: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self._request(url, |r| r.basic_auth(username, Some(password)))
            .await
    }
}

#[async_trait]
impl GitHubClient for RealGitHubClient {
    async fn current_user(&self, auth: &AccessToken) -> Result<GitHubUser> {
        self.request("/user", auth).await
    }

    async fn get_user(&self, name: &str, auth: &AccessToken) -> Result<GitHubUser> {
        let url = format!("/users/{name}");
        self.request(&url, auth).await
    }

    async fn org_by_name(&self, org_name: &str, auth: &AccessToken) -> Result<GitHubOrganization> {
        let url = format!("/orgs/{org_name}");
        self.request(&url, auth).await
    }

    async fn team_by_name(
        &self,
        org_name: &str,
        team_name: &str,
        auth: &AccessToken,
    ) -> Result<GitHubTeam> {
        let url = format!("/orgs/{org_name}/teams/{team_name}");
        self.request(&url, auth).await
    }

    async fn team_membership(
        &self,
        org_id: i32,
        team_id: i32,
        username: &str,
        auth: &AccessToken,
    ) -> Result<Option<GitHubTeamMembership>> {
        let url = format!("/organizations/{org_id}/team/{team_id}/memberships/{username}");
        match self.request(&url, auth).await {
            Ok(membership) => Ok(Some(membership)),
            // Officially how `false` is returned
            Err(GitHubError::NotFound(_)) => Ok(None),
            Err(err) => Err(err),
        }
    }

    async fn org_membership(
        &self,
        org_id: i32,
        username: &str,
        auth: &AccessToken,
    ) -> Result<Option<GitHubOrgMembership>> {
        let url = format!("/organizations/{org_id}/memberships/{username}");
        match self.request(&url, auth).await {
            Ok(membership) => Ok(Some(membership)),
            Err(GitHubError::NotFound(_)) => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Returns the list of public keys that can be used to verify GitHub secret alert signatures
    async fn public_keys(&self, username: &str, password: &str) -> Result<Vec<GitHubPublicKey>> {
        let url = "/meta/public_keys/secret_scanning";
        match self
            .request_basic::<GitHubPublicKeyList>(url, username, password)
            .await
        {
            Ok(v) => Ok(v.public_keys),
            Err(e) => Err(e),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GitHubError {
    #[error(transparent)]
    Permission(anyhow::Error),
    #[error(transparent)]
    NotFound(anyhow::Error),
    #[error(transparent)]
    Other(anyhow::Error),
}

impl From<reqwest::Error> for GitHubError {
    fn from(error: reqwest::Error) -> Self {
        use reqwest::StatusCode as Status;

        match error.status() {
            Some(Status::UNAUTHORIZED) | Some(Status::FORBIDDEN) => Self::Permission(error.into()),
            Some(Status::NOT_FOUND) => Self::NotFound(error.into()),
            _ => Self::Other(error.into()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GitHubUser {
    pub avatar_url: Option<String>,
    pub email: Option<String>,
    pub id: i32,
    pub login: String,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubOrganization {
    pub id: i32, // unique GH id (needed for membership queries)
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubTeam {
    pub id: i32,              // unique GH id (needed for membership queries)
    pub name: Option<String>, // Pretty name
    pub organization: GitHubOrganization,
}

#[derive(Debug, Deserialize)]
pub struct GitHubTeamMembership {
    pub state: String,
}

impl GitHubTeamMembership {
    pub fn is_active(&self) -> bool {
        self.state == "active"
    }
}

#[derive(Debug, Deserialize)]
pub struct GitHubOrgMembership {
    pub state: String,
    pub role: String,
}

impl GitHubOrgMembership {
    pub fn is_active_admin(&self) -> bool {
        self.state == "active" && self.role == "admin"
    }
}

#[derive(Debug, Deserialize, Clone, Eq, Hash, PartialEq)]
pub struct GitHubPublicKey {
    pub key_identifier: String,
    pub key: String,
    pub is_current: bool,
}

#[derive(Debug, Deserialize)]
pub struct GitHubPublicKeyList {
    pub public_keys: Vec<GitHubPublicKey>,
}

pub fn team_url(login: &str) -> String {
    let mut login_pieces = login.split(':');
    login_pieces.next();
    format!(
        "https://github.com/{}",
        login_pieces.next().expect("org failed"),
    )
}
