use crate::models::{Owner, User};
use crates_io_github::{GitHubClient, GitHubError};
use oauth2::AccessToken;
use secrecy::ExposeSecret;

/// Access rights to the crate (publishing and ownership management)
/// NOTE: The order of these variants matters!
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub enum Rights {
    None,
    Publish,
    Full,
}

impl Rights {
    /// Given this set of owners, determines the strongest rights the
    /// user has.
    ///
    /// Short-circuits on `Full` because you can't beat it. In practice, we'll always
    /// see `[user, user, user, ..., team, team, team]`, so we could shortcircuit on
    /// `Publish` as well, but this is a non-obvious invariant so we don't bother.
    /// Sweet free optimization if teams are proving burdensome to check.
    /// More than one team isn't really expected, though.
    pub async fn get(
        user: &User,
        gh_client: &dyn GitHubClient,
        owners: &[Owner],
    ) -> Result<Self, GitHubError> {
        let token = AccessToken::new(user.gh_access_token.expose_secret().to_string());

        let mut best = Self::None;
        for owner in owners {
            match *owner {
                Owner::User(ref other_user) => {
                    if other_user.id == user.id {
                        return Ok(Self::Full);
                    }
                }
                Owner::Team(ref team) => {
                    // Phones home to GitHub to ask if this User is a member of the given team.
                    // Note that we're assuming that the given user is the one interested in
                    // the answer. If this is not the case, then we could accidentally leak
                    // private membership information here.
                    let is_team_member = gh_client
                        .team_membership(team.org_id, team.github_id, &user.gh_login, &token)
                        .await?
                        .is_some_and(|m| m.is_active());

                    if is_team_member {
                        best = Self::Publish;
                    }
                }
            }
        }
        Ok(best)
    }
}
