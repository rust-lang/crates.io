use anyhow::anyhow;
use crates_io_github::{
    GitHubError, GitHubOrgMembership, GitHubOrganization, GitHubPublicKey, GitHubTeam,
    GitHubTeamMembership, GithubUser, MockGitHubClient,
};
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

impl MockData {
    pub fn as_mock_client(&'static self) -> MockGitHubClient {
        let mut mock = MockGitHubClient::new();

        mock.expect_current_user()
            .returning(|_auth| self.current_user());

        mock.expect_org_by_name()
            .returning(|org_name, _auth| self.org_by_name(org_name));

        mock.expect_team_by_name()
            .returning(|org_name, team_name, _auth| self.team_by_name(org_name, team_name));

        mock.expect_team_membership()
            .returning(|org_id, team_id, username, _auth| {
                self.team_membership(org_id, team_id, username)
            });

        mock.expect_org_membership()
            .returning(|org_id, username, _auth| self.org_membership(org_id, username));

        mock.expect_public_keys()
            .returning(|_username, _password| self.public_keys());

        mock
    }

    fn current_user(&self) -> Result<GithubUser, GitHubError> {
        let user = &self.users[0];
        Ok(GithubUser {
            id: user.id,
            login: user.login.into(),
            name: Some(user.name.into()),
            email: Some(user.email.into()),
            avatar_url: Some(format!("https://avatars.example.com/{}", user.id)),
        })
    }

    fn org_by_name(&self, org_name: &str) -> Result<GitHubOrganization, GitHubError> {
        let org = self
            .orgs
            .iter()
            .find(|org| org.name == org_name.to_lowercase())
            .ok_or_else(not_found)?;
        Ok(GitHubOrganization {
            id: org.id,
            avatar_url: Some(format!("https://avatars.example.com/o/{}", org.id)),
        })
    }

    fn team_by_name(&self, org_name: &str, team_name: &str) -> Result<GitHubTeam, GitHubError> {
        let team = self
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
            organization: self.org_by_name(org_name)?,
        })
    }

    fn team_membership(
        &self,
        org_id: i32,
        team_id: i32,
        username: &str,
    ) -> Result<GitHubTeamMembership, GitHubError> {
        let team = self
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

    fn org_membership(
        &self,
        org_id: i32,
        username: &str,
    ) -> Result<GitHubOrgMembership, GitHubError> {
        let org = self
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

    fn public_keys(&self) -> Result<Vec<GitHubPublicKey>, GitHubError> {
        Ok(self.public_keys.iter().map(Into::into).collect())
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
