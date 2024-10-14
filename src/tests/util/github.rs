use anyhow::anyhow;
use async_trait::async_trait;
use crates_io_github::{
    GitHubClient, GitHubError, GitHubOrgMembership, GitHubOrganization, GitHubPublicKey,
    GitHubTeam, GitHubTeamMembership, GithubUser,
};
use oauth2::AccessToken;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_GH_ID: AtomicUsize = AtomicUsize::new(0);

pub fn next_gh_id() -> i32 {
    NEXT_GH_ID.fetch_add(1, Ordering::SeqCst) as i32
}

pub(crate) const MOCK_GITHUB_DATA: MockData = MockData {
    orgs: &[MockOrg {
        id: 1000,
        name: "test-org",
        owners: &["user-org-owner"],
        teams: &[
            MockTeam {
                id: 2000,
                name: "all",
                members: &["user-all-teams", "user-one-team"],
            },
            MockTeam {
                id: 2001,
                name: "core",
                members: &["user-all-teams"],
            },
        ],
    }],
    users: &[
        MockUser {
            id: 1,
            login: "user-one-team",
            name: "User on one team",
            email: "one-team@example.com",
        },
        MockUser {
            id: 2,
            login: "user-all-teams",
            name: "User on all teams",
            email: "all-teams@example.com",
        },
        MockUser {
            id: 3,
            login: "user-org-owner",
            name: "User owning the org",
            email: "owner@example.com",
        },
    ],
    // Test key from https://docs.github.com/en/developers/overview/secret-scanning-partner-program#create-a-secret-alert-service
    public_keys: &[
        MockPublicKey {
            key_identifier: "f9525bf080f75b3506ca1ead061add62b8633a346606dc5fe544e29231c6ee0d",
            key: "-----BEGIN PUBLIC KEY-----\nMFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEsz9ugWDj5jK5ELBK42ynytbo38gP\nHzZFI03Exwz8Lh/tCfL3YxwMdLjB+bMznsanlhK0RwcGP3IDb34kQDIo3Q==\n-----END PUBLIC KEY-----",
            is_current: true,
        },
    ],
};

pub(crate) struct MockGitHubClient {
    data: &'static MockData,
}

impl MockGitHubClient {
    pub(crate) fn new(data: &'static MockData) -> Self {
        Self { data }
    }
}

#[async_trait]
impl GitHubClient for MockGitHubClient {
    async fn current_user(&self, _auth: &AccessToken) -> Result<GithubUser, GitHubError> {
        let user = &self.data.users[0];
        Ok(GithubUser {
            id: user.id,
            login: user.login.into(),
            name: Some(user.name.into()),
            email: Some(user.email.into()),
            avatar_url: Some(format!("https://avatars.example.com/{}", user.id)),
        })
    }

    async fn org_by_name(
        &self,
        org_name: &str,
        _auth: &AccessToken,
    ) -> Result<GitHubOrganization, GitHubError> {
        let org = self
            .data
            .orgs
            .iter()
            .find(|org| org.name == org_name.to_lowercase())
            .ok_or_else(not_found)?;
        Ok(GitHubOrganization {
            id: org.id,
            avatar_url: Some(format!("https://avatars.example.com/o/{}", org.id)),
        })
    }

    async fn team_by_name(
        &self,
        org_name: &str,
        team_name: &str,
        auth: &AccessToken,
    ) -> Result<GitHubTeam, GitHubError> {
        let team = self
            .data
            .orgs
            .iter()
            .find(|org| org.name == org_name.to_lowercase())
            .ok_or_else(not_found)?
            .teams
            .iter()
            .find(|team| team.name == team_name.to_lowercase())
            .ok_or_else(not_found)?;
        Ok(GitHubTeam {
            id: team.id,
            name: Some(team.name.into()),
            organization: self.org_by_name(org_name, auth).await?,
        })
    }

    async fn team_membership(
        &self,
        org_id: i32,
        team_id: i32,
        username: &str,
        _auth: &AccessToken,
    ) -> Result<GitHubTeamMembership, GitHubError> {
        let team = self
            .data
            .orgs
            .iter()
            .find(|org| org.id == org_id)
            .ok_or_else(not_found)?
            .teams
            .iter()
            .find(|team| team.id == team_id)
            .ok_or_else(not_found)?;
        if team.members.contains(&username) {
            Ok(GitHubTeamMembership {
                state: "active".into(),
            })
        } else {
            Err(not_found())
        }
    }

    async fn org_membership(
        &self,
        org_id: i32,
        username: &str,
        _auth: &AccessToken,
    ) -> Result<GitHubOrgMembership, GitHubError> {
        let org = self
            .data
            .orgs
            .iter()
            .find(|org| org.id == org_id)
            .ok_or_else(not_found)?;
        if org.owners.contains(&username) {
            Ok(GitHubOrgMembership {
                state: "active".into(),
                role: "admin".into(),
            })
        } else if org
            .teams
            .iter()
            .any(|team| team.members.contains(&username))
        {
            Ok(GitHubOrgMembership {
                state: "active".into(),
                role: "member".into(),
            })
        } else {
            Err(not_found())
        }
    }

    async fn public_keys(
        &self,
        _username: &str,
        _password: &str,
    ) -> Result<Vec<GitHubPublicKey>, GitHubError> {
        Ok(self.data.public_keys.iter().map(Into::into).collect())
    }
}

fn not_found() -> GitHubError {
    GitHubError::NotFound(anyhow!("404"))
}

pub(crate) struct MockData {
    orgs: &'static [MockOrg],
    users: &'static [MockUser],
    public_keys: &'static [MockPublicKey],
}

struct MockUser {
    id: i32,
    login: &'static str,
    name: &'static str,
    email: &'static str,
}

struct MockOrg {
    id: i32,
    name: &'static str,
    owners: &'static [&'static str],
    teams: &'static [MockTeam],
}

struct MockTeam {
    id: i32,
    name: &'static str,
    members: &'static [&'static str],
}

struct MockPublicKey {
    key_identifier: &'static str,
    key: &'static str,
    is_current: bool,
}

impl From<&'static MockPublicKey> for GitHubPublicKey {
    fn from(k: &'static MockPublicKey) -> Self {
        Self {
            key_identifier: k.key_identifier.to_string(),
            key: k.key.to_string(),
            is_current: k.is_current,
        }
    }
}
