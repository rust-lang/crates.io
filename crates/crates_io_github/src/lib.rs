#![doc = include_str!("../README.md")]

#[macro_use]
extern crate tracing;

mod slug;

pub use crate::slug::{ParseSlugError, parse_github_slug};

use oauth2::AccessToken;
use reqwest::{self, RequestBuilder, header};

use secrecy::{ExposeSecret, SecretString};
use serde::de::DeserializeOwned;

use std::str;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;

type Result<T> = std::result::Result<T, GitHubError>;

/// Authentication mode for a request to the GitHub API.
///
/// Each [`GitHubClient`] method takes a `&GitHubAuth` so the
/// authentication mode is decided at the call site rather than baked into
/// the method signature.
#[derive(Debug, Clone)]
pub enum GitHubAuth {
    /// Unauthenticated request. Used for reads against public
    /// repositories where the unauthenticated rate limit is sufficient.
    None,
    /// OAuth/installation bearer token authentication.
    Bearer { token: AccessToken },
    /// HTTP basic authentication, used for the secret-scanning public key
    /// endpoint where the OAuth client id/secret act as the credentials.
    Basic {
        username: String,
        password: SecretString,
    },
}

impl GitHubAuth {
    /// Creates a [`GitHubAuth::Bearer`] authentication mode from a bearer
    /// token.
    pub fn bearer(token: AccessToken) -> Self {
        GitHubAuth::Bearer { token }
    }

    /// Creates an [`GitHubAuth::Basic`] authentication mode from a username
    /// and password.
    pub fn basic(username: impl Into<String>, password: impl Into<SecretString>) -> Self {
        GitHubAuth::Basic {
            username: username.into(),
            password: password.into(),
        }
    }

    /// Applies this authentication mode to the given request builder.
    fn apply(&self, request: RequestBuilder) -> RequestBuilder {
        match self {
            GitHubAuth::None => request,
            GitHubAuth::Bearer { token } => request.bearer_auth(token.secret()),
            GitHubAuth::Basic { username, password } => {
                request.basic_auth(username, Some(password.expose_secret()))
            }
        }
    }
}

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait GitHubClient: Send + Sync {
    async fn current_user(&self, auth: &GitHubAuth) -> Result<GitHubUser>;
    async fn get_user(&self, name: &str, auth: &GitHubAuth) -> Result<GitHubUser>;
    async fn get_user_by_id(&self, account_id: i64, auth: &GitHubAuth) -> Result<GitHubUser>;
    async fn org_by_name(&self, org_name: &str, auth: &GitHubAuth) -> Result<GitHubOrganization>;
    async fn team_by_name(
        &self,
        org_name: &str,
        team_name: &str,
        auth: &GitHubAuth,
    ) -> Result<GitHubTeam>;
    async fn team_membership(
        &self,
        org_id: i32,
        team_id: i32,
        username: &str,
        auth: &GitHubAuth,
    ) -> Result<Option<GitHubTeamMembership>>;
    async fn org_membership(
        &self,
        org_id: i32,
        username: &str,
        auth: &GitHubAuth,
    ) -> Result<Option<GitHubOrgMembership>>;

    /// Returns the list of public keys that can be used to verify GitHub
    /// secret alert signatures.
    async fn public_keys(&self, auth: &GitHubAuth) -> Result<Vec<GitHubPublicKey>>;

    /// Fetches a single git ref.
    ///
    /// `ref_name` may be given either fully qualified (e.g.
    /// `"refs/heads/master"`) or without the `refs/` prefix (e.g.
    /// `"heads/master"`).
    async fn get_ref(
        &self,
        owner: &str,
        repo: &str,
        ref_name: &str,
        auth: &GitHubAuth,
    ) -> Result<GitRef>;

    /// Fetches a single commit object by its SHA.
    async fn get_commit(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
        auth: &GitHubAuth,
    ) -> Result<GitCommit>;

    /// Creates a new commit object in the given repository.
    ///
    /// Passing an empty `parents` slice produces a parentless root
    /// commit. The returned [`GitCommit`] contains the newly assigned
    /// SHA.
    async fn create_commit<'a>(
        &self,
        owner: &str,
        repo: &str,
        input: &CreateCommit<'a>,
        auth: &GitHubAuth,
    ) -> Result<GitCommit>;

    /// Creates a new git ref.
    ///
    /// `ref_name` must be the fully qualified form (e.g.
    /// `"refs/heads/my-branch"`).
    async fn create_ref(
        &self,
        owner: &str,
        repo: &str,
        ref_name: &str,
        sha: &str,
        auth: &GitHubAuth,
    ) -> Result<GitRef>;

    /// Updates an existing git ref to point at `sha`.
    ///
    /// `ref_name` may be given either fully qualified or without the
    /// `refs/` prefix, matching [`GitHubClient::get_ref`]. Set `force`
    /// to allow non-fast-forward updates.
    async fn update_ref(
        &self,
        owner: &str,
        repo: &str,
        ref_name: &str,
        sha: &str,
        force: bool,
        auth: &GitHubAuth,
    ) -> Result<GitRef>;
}

#[derive(Debug)]
pub struct RealGitHubClient {
    client: Client,
    base_url: Url,
}

impl RealGitHubClient {
    pub fn new(client: Client) -> Self {
        let base_url = Url::parse("https://api.github.com").expect("base URL must parse");
        Self::with_base_url(client, base_url)
    }

    fn with_base_url(client: Client, base_url: Url) -> Self {
        Self { client, base_url }
    }

    /// Does all the nonsense for sending a GET to GitHub.
    async fn request<T>(&self, url: &str, auth: &GitHubAuth) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = self
            .base_url
            .join(url.trim_start_matches('/'))
            .map_err(|e| GitHubError::Other(e.into()))?;
        info!("GitHub request: GET {url}");

        let request = self
            .client
            .get(url)
            .header(header::ACCEPT, "application/vnd.github.v3+json")
            .header(header::USER_AGENT, "crates.io (https://crates.io)");

        let response = auth.apply(request).send().await?.error_for_status()?;

        let headers = response.headers();
        let remaining = headers.get("x-ratelimit-remaining");
        let limit = headers.get("x-ratelimit-limit");
        debug!("GitHub rate limit remaining: {remaining:?}/{limit:?}");

        response.json().await.map_err(Into::into)
    }

    /// Sends a request with a JSON body to GitHub.
    async fn _mutate<B, T>(
        &self,
        method: reqwest::Method,
        url: &str,
        body: &B,
        auth: &GitHubAuth,
    ) -> Result<T>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let url = self
            .base_url
            .join(url.trim_start_matches('/'))
            .map_err(|e| GitHubError::Other(e.into()))?;
        info!("GitHub request: {method} {url}");

        let request = self
            .client
            .request(method, url)
            .header(header::ACCEPT, "application/vnd.github.v3+json")
            .header(header::USER_AGENT, "crates.io (https://crates.io)")
            .json(body);

        let response = auth.apply(request).send().await?.error_for_status()?;

        let headers = response.headers();
        let remaining = headers.get("x-ratelimit-remaining");
        let limit = headers.get("x-ratelimit-limit");
        debug!("GitHub rate limit remaining: {remaining:?}/{limit:?}");

        response.json().await.map_err(Into::into)
    }
}

#[async_trait]
impl GitHubClient for RealGitHubClient {
    async fn current_user(&self, auth: &GitHubAuth) -> Result<GitHubUser> {
        self.request("/user", auth).await
    }

    async fn get_user(&self, name: &str, auth: &GitHubAuth) -> Result<GitHubUser> {
        let url = format!("/users/{name}");
        self.request(&url, auth).await
    }

    async fn get_user_by_id(&self, account_id: i64, auth: &GitHubAuth) -> Result<GitHubUser> {
        let url = format!("/user/{account_id}");
        self.request(&url, auth).await
    }

    async fn org_by_name(&self, org_name: &str, auth: &GitHubAuth) -> Result<GitHubOrganization> {
        let url = format!("/orgs/{org_name}");
        self.request(&url, auth).await
    }

    async fn team_by_name(
        &self,
        org_name: &str,
        team_name: &str,
        auth: &GitHubAuth,
    ) -> Result<GitHubTeam> {
        let url = format!("/orgs/{org_name}/teams/{team_name}");
        self.request(&url, auth).await
    }

    async fn team_membership(
        &self,
        org_id: i32,
        team_id: i32,
        username: &str,
        auth: &GitHubAuth,
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
        auth: &GitHubAuth,
    ) -> Result<Option<GitHubOrgMembership>> {
        let url = format!("/organizations/{org_id}/memberships/{username}");
        match self.request(&url, auth).await {
            Ok(membership) => Ok(Some(membership)),
            Err(GitHubError::NotFound(_)) => Ok(None),
            Err(err) => Err(err),
        }
    }

    async fn public_keys(&self, auth: &GitHubAuth) -> Result<Vec<GitHubPublicKey>> {
        let url = "/meta/public_keys/secret_scanning";
        match self.request::<GitHubPublicKeyList>(url, auth).await {
            Ok(v) => Ok(v.public_keys),
            Err(e) => Err(e),
        }
    }

    async fn get_ref(
        &self,
        owner: &str,
        repo: &str,
        ref_name: &str,
        auth: &GitHubAuth,
    ) -> Result<GitRef> {
        let ref_path = ref_name.strip_prefix("refs/").unwrap_or(ref_name);
        let path = format!("/repos/{owner}/{repo}/git/ref/{ref_path}");
        self.request(&path, auth).await
    }

    async fn get_commit(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
        auth: &GitHubAuth,
    ) -> Result<GitCommit> {
        let path = format!("/repos/{owner}/{repo}/git/commits/{sha}");
        self.request(&path, auth).await
    }

    async fn create_commit<'a>(
        &self,
        owner: &str,
        repo: &str,
        input: &CreateCommit<'a>,
        auth: &GitHubAuth,
    ) -> Result<GitCommit> {
        let path = format!("/repos/{owner}/{repo}/git/commits");
        self._mutate(reqwest::Method::POST, &path, input, auth)
            .await
    }

    async fn create_ref(
        &self,
        owner: &str,
        repo: &str,
        ref_name: &str,
        sha: &str,
        auth: &GitHubAuth,
    ) -> Result<GitRef> {
        #[derive(Serialize)]
        struct Body<'a> {
            #[serde(rename = "ref")]
            ref_name: &'a str,
            sha: &'a str,
        }

        let path = format!("/repos/{owner}/{repo}/git/refs");
        let body = Body { ref_name, sha };
        self._mutate(reqwest::Method::POST, &path, &body, auth)
            .await
    }

    async fn update_ref(
        &self,
        owner: &str,
        repo: &str,
        ref_name: &str,
        sha: &str,
        force: bool,
        auth: &GitHubAuth,
    ) -> Result<GitRef> {
        #[derive(Serialize)]
        struct Body<'a> {
            sha: &'a str,
            force: bool,
        }

        let ref_path = ref_name.strip_prefix("refs/").unwrap_or(ref_name);
        let path = format!("/repos/{owner}/{repo}/git/refs/{ref_path}");
        let body = Body { sha, force };
        self._mutate(reqwest::Method::PATCH, &path, &body, auth)
            .await
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GitHubError {
    #[error(transparent)]
    Unauthorized(anyhow::Error),
    #[error(transparent)]
    Forbidden(anyhow::Error),
    #[error(transparent)]
    NotFound(anyhow::Error),
    #[error(transparent)]
    Other(anyhow::Error),
}

impl From<reqwest::Error> for GitHubError {
    fn from(error: reqwest::Error) -> Self {
        use reqwest::StatusCode as Status;

        match error.status() {
            Some(Status::UNAUTHORIZED) => Self::Unauthorized(error.into()),
            Some(Status::FORBIDDEN) => Self::Forbidden(error.into()),
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

/// A git ref on GitHub.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GitRef {
    /// The fully qualified ref name (e.g. `"refs/heads/master"`).
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub object: GitObject,
}

/// A git object referenced from a ref or commit.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GitObject {
    pub sha: String,
}

/// A git commit on GitHub.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GitCommit {
    pub sha: String,
    pub tree: GitObject,
}

/// Input payload for [`GitHubClient::create_commit`].
///
/// An empty `parents` slice produces a parentless root commit.
#[derive(Debug, Serialize)]
pub struct CreateCommit<'a> {
    pub message: &'a str,
    pub tree: &'a str,
    pub parents: &'a [&'a str],
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server, ServerOpts};
    use serde_json::json;

    async fn mock_server() -> Server {
        Server::new_with_opts_async(ServerOpts {
            assert_on_drop: true,
            ..Default::default()
        })
        .await
    }

    fn client_with_server(server: &Server) -> RealGitHubClient {
        let base_url = Url::parse(&server.url()).unwrap();
        RealGitHubClient::with_base_url(Client::new(), base_url)
    }

    const USER_BODY: &str = r#"{
        "avatar_url": "https://avatars.githubusercontent.com/u/1?v=4",
        "email": null,
        "id": 1,
        "login": "johnnydee",
        "name": "John Doe"
    }"#;

    const REF_BODY: &str = r#"{
        "ref": "refs/heads/master",
        "node_id": "abc",
        "url": "https://api.github.com/ignored",
        "object": {
            "type": "commit",
            "sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "url": "https://api.github.com/ignored"
        }
    }"#;

    const COMMIT_BODY: &str = r#"{
        "sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "node_id": "abc",
        "url": "https://api.github.com/ignored",
        "html_url": "https://github.com/ignored",
        "author": {"name": "bors", "email": "bors@rust-lang.org", "date": "2026-04-24T00:00:00Z"},
        "committer": {"name": "bors", "email": "bors@rust-lang.org", "date": "2026-04-24T00:00:00Z"},
        "tree": {
            "sha": "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
            "url": "https://api.github.com/ignored"
        },
        "message": "ignored",
        "parents": []
    }"#;

    #[tokio::test]
    async fn get_user_hits_configured_base_url() {
        let mut server = mock_server().await;
        let _mock = server
            .mock("GET", "/users/johnnydee")
            .match_header("authorization", "Bearer test-token")
            .with_status(200)
            .with_body(USER_BODY)
            .expect(1)
            .create_async()
            .await;

        let client = client_with_server(&server);
        let auth = GitHubAuth::bearer(AccessToken::new("test-token".into()));
        let user = client.get_user("johnnydee", &auth).await.unwrap();

        assert_eq!(user.login, "johnnydee");
        assert_eq!(user.id, 1);
    }

    #[tokio::test]
    async fn get_ref_strips_refs_prefix_and_returns_sha() {
        let mut server = mock_server().await;
        let _mock = server
            .mock(
                "GET",
                "/repos/rust-lang/crates.io-index/git/ref/heads/master",
            )
            .match_header("accept", "application/vnd.github.v3+json")
            .match_header("user-agent", "crates.io (https://crates.io)")
            .with_status(200)
            .with_body(REF_BODY)
            .expect(1)
            .create_async()
            .await;

        let client = client_with_server(&server);
        let auth = GitHubAuth::None;
        let got = client
            .get_ref("rust-lang", "crates.io-index", "refs/heads/master", &auth)
            .await
            .unwrap();

        assert_eq!(got.ref_name, "refs/heads/master");
        assert_eq!(got.object.sha, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    }

    #[tokio::test]
    async fn get_ref_accepts_unqualified_ref_name() {
        let mut server = mock_server().await;
        let _mock = server
            .mock(
                "GET",
                "/repos/rust-lang/crates.io-index/git/ref/heads/master",
            )
            .with_status(200)
            .with_body(REF_BODY)
            .expect(1)
            .create_async()
            .await;

        let client = client_with_server(&server);
        let auth = GitHubAuth::None;
        let got = client
            .get_ref("rust-lang", "crates.io-index", "heads/master", &auth)
            .await
            .unwrap();

        assert_eq!(got.ref_name, "refs/heads/master");
    }

    #[tokio::test]
    async fn get_commit_returns_sha_and_tree_sha() {
        let mut server = mock_server().await;
        let sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let _mock = server
            .mock(
                "GET",
                format!("/repos/rust-lang/crates.io-index/git/commits/{sha}").as_str(),
            )
            .match_header("accept", "application/vnd.github.v3+json")
            .with_status(200)
            .with_body(COMMIT_BODY)
            .expect(1)
            .create_async()
            .await;

        let client = client_with_server(&server);
        let auth = GitHubAuth::None;
        let got = client
            .get_commit("rust-lang", "crates.io-index", sha, &auth)
            .await
            .unwrap();

        assert_eq!(got.sha, sha);
        assert_eq!(got.tree.sha, "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
    }

    #[tokio::test]
    async fn create_commit_posts_body_and_returns_commit() {
        let parent = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let new_sha = "cccccccccccccccccccccccccccccccccccccccc";
        let tree = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
        let response = format!(
            r#"{{
                "sha": "{new_sha}",
                "tree": {{"sha": "{tree}", "url": "https://api.github.com/ignored"}},
                "message": "collapse",
                "parents": [
                    {{"sha": "{parent}", "url": "https://api.github.com/ignored"}}
                ]
            }}"#
        );

        let mut server = mock_server().await;
        let _mock = server
            .mock("POST", "/repos/rust-lang/crates.io-index/git/commits")
            .match_header("authorization", "Bearer test-token")
            .match_header("accept", "application/vnd.github.v3+json")
            .match_body(Matcher::Json(json!({
                "message": "collapse",
                "tree": tree,
                "parents": [parent],
            })))
            .with_status(201)
            .with_body(&response)
            .expect(1)
            .create_async()
            .await;

        let client = client_with_server(&server);
        let auth = GitHubAuth::bearer(AccessToken::new("test-token".into()));
        let parents = [parent];
        let input = CreateCommit {
            message: "collapse",
            tree,
            parents: &parents,
        };

        let got = client
            .create_commit("rust-lang", "crates.io-index", &input, &auth)
            .await
            .unwrap();

        assert_eq!(got.sha, new_sha);
        assert_eq!(got.tree.sha, tree);
    }

    #[tokio::test]
    async fn create_ref_sends_fully_qualified_ref() {
        let ref_name = "refs/heads/snapshot-2026-04-24";
        let sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let response = format!(
            r#"{{
                "ref": "{ref_name}",
                "object": {{
                    "type": "commit",
                    "sha": "{sha}",
                    "url": "https://api.github.com/ignored"
                }}
            }}"#
        );

        let mut server = mock_server().await;
        let _mock = server
            .mock("POST", "/repos/rust-lang/crates.io-index/git/refs")
            .match_header("authorization", "Bearer test-token")
            .match_body(Matcher::Json(json!({
                "ref": ref_name,
                "sha": sha,
            })))
            .with_status(201)
            .with_body(&response)
            .expect(1)
            .create_async()
            .await;

        let client = client_with_server(&server);
        let auth = GitHubAuth::bearer(AccessToken::new("test-token".into()));

        let got = client
            .create_ref("rust-lang", "crates.io-index", ref_name, sha, &auth)
            .await
            .unwrap();

        assert_eq!(got.ref_name, ref_name);
        assert_eq!(got.object.sha, sha);
    }

    #[tokio::test]
    async fn update_ref_strips_refs_prefix_and_sends_force_flag() {
        let new_sha = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let response = format!(
            r#"{{
                "ref": "refs/heads/master",
                "object": {{
                    "type": "commit",
                    "sha": "{new_sha}",
                    "url": "https://api.github.com/ignored"
                }}
            }}"#
        );

        let mut server = mock_server().await;
        let _mock = server
            .mock(
                "PATCH",
                "/repos/rust-lang/crates.io-index/git/refs/heads/master",
            )
            .match_header("authorization", "Bearer test-token")
            .match_body(Matcher::Json(json!({
                "sha": new_sha,
                "force": true,
            })))
            .with_status(200)
            .with_body(&response)
            .expect(1)
            .create_async()
            .await;

        let client = client_with_server(&server);
        let auth = GitHubAuth::bearer(AccessToken::new("test-token".into()));

        let got = client
            .update_ref(
                "rust-lang",
                "crates.io-index",
                "refs/heads/master",
                new_sha,
                true,
                &auth,
            )
            .await
            .unwrap();

        assert_eq!(got.ref_name, "refs/heads/master");
        assert_eq!(got.object.sha, new_sha);
    }

    #[tokio::test]
    async fn none_auth_sends_no_authorization_header() {
        let mut server = mock_server().await;
        let _mock = server
            .mock("GET", "/users/johnnydee")
            .match_header("authorization", Matcher::Missing)
            .with_status(200)
            .with_body(USER_BODY)
            .expect(1)
            .create_async()
            .await;

        let client = client_with_server(&server);
        let user = client
            .get_user("johnnydee", &GitHubAuth::None)
            .await
            .unwrap();

        assert_eq!(user.login, "johnnydee");
    }

    #[tokio::test]
    async fn token_auth_sends_bearer_header() {
        let mut server = mock_server().await;
        let _mock = server
            .mock("GET", "/users/johnnydee")
            .match_header("authorization", "Bearer test-token")
            .with_status(200)
            .with_body(USER_BODY)
            .expect(1)
            .create_async()
            .await;

        let client = client_with_server(&server);
        let auth = GitHubAuth::bearer(AccessToken::new("test-token".into()));
        let user = client.get_user("johnnydee", &auth).await.unwrap();

        assert_eq!(user.login, "johnnydee");
    }

    #[tokio::test]
    async fn basic_auth_sends_basic_header() {
        // base64("client-id:client-secret")
        let expected = "Basic Y2xpZW50LWlkOmNsaWVudC1zZWNyZXQ=";
        let mut server = mock_server().await;
        let _mock = server
            .mock("GET", "/users/johnnydee")
            .match_header("authorization", expected)
            .with_status(200)
            .with_body(USER_BODY)
            .expect(1)
            .create_async()
            .await;

        let client = client_with_server(&server);
        let auth = GitHubAuth::basic("client-id", "client-secret");
        let user = client.get_user("johnnydee", &auth).await.unwrap();

        assert_eq!(user.login, "johnnydee");
    }
}
