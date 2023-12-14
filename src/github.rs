//! This module implements functionality for interacting with GitHub.

use oauth2::AccessToken;
use reqwest::{self, header};

use serde::de::DeserializeOwned;

use std::str;

use crate::controllers::github::secret_scanning::{GitHubPublicKey, GitHubPublicKeyList};
use crate::util::errors::{cargo_err, internal, not_found, AppResult, BoxedAppError};
use async_trait::async_trait;
use reqwest::Client;

#[async_trait]
pub trait GitHubClient: Send + Sync {
    async fn current_user(&self, auth: &AccessToken) -> AppResult<GithubUser>;
    async fn org_by_name(
        &self,
        org_name: &str,
        auth: &AccessToken,
    ) -> AppResult<GitHubOrganization>;
    async fn team_by_name(
        &self,
        org_name: &str,
        team_name: &str,
        auth: &AccessToken,
    ) -> AppResult<GitHubTeam>;
    async fn team_membership(
        &self,
        org_id: i32,
        team_id: i32,
        username: &str,
        auth: &AccessToken,
    ) -> AppResult<GitHubTeamMembership>;
    async fn org_membership(
        &self,
        org_id: i32,
        username: &str,
        auth: &AccessToken,
    ) -> AppResult<GitHubOrgMembership>;
    async fn public_keys(&self, username: &str, password: &str) -> AppResult<Vec<GitHubPublicKey>>;
}

#[derive(Debug)]
pub struct RealGitHubClient {
    client: Client,
}

impl RealGitHubClient {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Does all the nonsense for sending a GET to Github.
    async fn _request<T>(&self, url: &str, auth: &str) -> AppResult<T>
    where
        T: DeserializeOwned,
    {
        let url = format!("https://api.github.com{url}");
        info!("GITHUB HTTP: {url}");

        self.client
            .get(&url)
            .header(header::ACCEPT, "application/vnd.github.v3+json")
            .header(header::AUTHORIZATION, auth)
            .header(header::USER_AGENT, "crates.io (https://crates.io)")
            .send()
            .await?
            .error_for_status()
            .map_err(|e| handle_error_response(&e))?
            .json()
            .await
            .map_err(Into::into)
    }

    /// Sends a GET to GitHub using OAuth access token authentication
    pub async fn request<T>(&self, url: &str, auth: &AccessToken) -> AppResult<T>
    where
        T: DeserializeOwned,
    {
        self._request(url, &format!("Bearer {}", auth.secret()))
            .await
    }

    /// Sends a GET to GitHub using basic authentication
    pub async fn request_basic<T>(&self, url: &str, username: &str, password: &str) -> AppResult<T>
    where
        T: DeserializeOwned,
    {
        self._request(url, &format!("basic {username}:{password}"))
            .await
    }
}

#[async_trait]
impl GitHubClient for RealGitHubClient {
    async fn current_user(&self, auth: &AccessToken) -> AppResult<GithubUser> {
        self.request("/user", auth).await
    }

    async fn org_by_name(
        &self,
        org_name: &str,
        auth: &AccessToken,
    ) -> AppResult<GitHubOrganization> {
        let url = format!("/orgs/{org_name}");
        self.request(&url, auth).await
    }

    async fn team_by_name(
        &self,
        org_name: &str,
        team_name: &str,
        auth: &AccessToken,
    ) -> AppResult<GitHubTeam> {
        let url = format!("/orgs/{org_name}/teams/{team_name}");
        self.request(&url, auth).await
    }

    async fn team_membership(
        &self,
        org_id: i32,
        team_id: i32,
        username: &str,
        auth: &AccessToken,
    ) -> AppResult<GitHubTeamMembership> {
        let url = format!("/organizations/{org_id}/team/{team_id}/memberships/{username}");
        self.request(&url, auth).await
    }

    async fn org_membership(
        &self,
        org_id: i32,
        username: &str,
        auth: &AccessToken,
    ) -> AppResult<GitHubOrgMembership> {
        self.request(
            &format!("/organizations/{org_id}/memberships/{username}"),
            auth,
        )
        .await
    }

    /// Returns the list of public keys that can be used to verify GitHub secret alert signatures
    async fn public_keys(&self, username: &str, password: &str) -> AppResult<Vec<GitHubPublicKey>> {
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

fn handle_error_response(error: &reqwest::Error) -> BoxedAppError {
    use reqwest::StatusCode as Status;

    match error.status() {
        Some(Status::UNAUTHORIZED) | Some(Status::FORBIDDEN) => cargo_err(
            "It looks like you don't have permission \
             to query a necessary property from GitHub \
             to complete this request. \
             You may need to re-authenticate on \
             crates.io to grant permission to read \
             GitHub org memberships.",
        ),
        Some(Status::NOT_FOUND) => not_found(),
        _ => internal(format!("didn't get a 200 result from github: {error}")),
    }
}

#[derive(Debug, Deserialize)]
pub struct GithubUser {
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

#[derive(Debug, Deserialize)]
pub struct GitHubOrgMembership {
    pub state: String,
    pub role: String,
}

pub fn team_url(login: &str) -> String {
    let mut login_pieces = login.split(':');
    login_pieces.next();
    format!(
        "https://github.com/{}",
        login_pieces.next().expect("org failed"),
    )
}
