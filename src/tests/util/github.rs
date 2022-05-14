use cargo_registry::github::{
    GitHubClient, GitHubOrganization, GitHubTeam, GitHubTeamMembership, GithubUser,
};
use cargo_registry::util::errors::{not_found, AppResult};
use oauth2::AccessToken;

pub(crate) const MOCK_GITHUB_DATA: MockData = MockData {
    orgs: &[MockOrg {
        id: 1000,
        name: "test-org",
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

impl GitHubClient for MockGitHubClient {
    fn current_user(&self, _auth: &AccessToken) -> AppResult<GithubUser> {
        let user = &self.data.users[0];
        Ok(GithubUser {
            id: user.id,
            login: user.login.into(),
            name: Some(user.name.into()),
            email: Some(user.email.into()),
            avatar_url: Some(format!("https://avatars.example.com/{}", user.id)),
        })
    }

    fn org_by_name(&self, org_name: &str, _auth: &AccessToken) -> AppResult<GitHubOrganization> {
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

    fn team_by_name(
        &self,
        org_name: &str,
        team_name: &str,
        auth: &AccessToken,
    ) -> AppResult<GitHubTeam> {
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
            organization: self.org_by_name(org_name, auth)?,
        })
    }

    fn team_membership(
        &self,
        org_id: i32,
        team_id: i32,
        username: &str,
        _auth: &AccessToken,
    ) -> AppResult<GitHubTeamMembership> {
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
}

pub(crate) struct MockData {
    orgs: &'static [MockOrg],
    users: &'static [MockUser],
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
    teams: &'static [MockTeam],
}

struct MockTeam {
    id: i32,
    name: &'static str,
    members: &'static [&'static str],
}
