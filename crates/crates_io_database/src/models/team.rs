use bon::Builder;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::models::{Crate, CrateOwner, Owner, OwnerKind};
use crate::schema::{crate_owners, teams};

/// For now, just a GitHub Team. Can be upgraded to other teams
/// later if desirable.
#[derive(Queryable, Identifiable, serde::Serialize, serde::Deserialize, Debug, Selectable)]
pub struct Team {
    /// Unique table id
    pub id: i32,
    /// "github:org:team"
    /// An opaque unique ID, that was at one point parsed out to query GitHub.
    /// We only query membership with github using the github_id, though.
    /// This is the only name we should ever talk to Cargo about.
    pub login: String,
    /// The GitHub API works on team ID numbers. This can change, if a team
    /// is deleted and then recreated with the same name!!!
    pub github_id: i32,
    /// Sugary goodness
    pub name: Option<String>,
    pub avatar: Option<String>,
    /// The GitHub Organization ID this team sits under
    pub org_id: i32,
}

#[derive(Insertable, AsChangeset, Debug, Builder)]
#[diesel(table_name = teams, check_for_backend(diesel::pg::Pg))]
pub struct NewTeam<'a> {
    pub login: &'a str,
    pub github_id: i32,
    pub name: Option<&'a str>,
    pub avatar: Option<&'a str>,
    pub org_id: i32,
}

impl NewTeam<'_> {
    pub async fn create_or_update(&self, conn: &mut AsyncPgConnection) -> QueryResult<Team> {
        use diesel::insert_into;

        insert_into(teams::table)
            .values(self)
            .on_conflict(teams::github_id)
            .do_update()
            .set(self)
            .returning(Team::as_returning())
            .get_result(conn)
            .await
    }
}

impl Team {
    pub async fn owning(krate: &Crate, conn: &mut AsyncPgConnection) -> QueryResult<Vec<Owner>> {
        let base_query = CrateOwner::belonging_to(krate).filter(crate_owners::deleted.eq(false));
        let teams = base_query
            .inner_join(teams::table)
            .select(Team::as_select())
            .filter(crate_owners::owner_kind.eq(OwnerKind::Team))
            .load(conn)
            .await?
            .into_iter()
            .map(Owner::Team);

        Ok(teams.collect())
    }

    /// Splits the login into provider, organization, and team name.
    ///
    /// Returns `None` if the login format is invalid.
    pub fn split_login(&self) -> Option<(&str, &str, &str)> {
        let (provider, rest) = self.login.split_once(':')?;
        let (org, team) = rest.split_once(':')?;
        Some((provider, org, team))
    }

    /// Returns the URL for the team.
    ///
    /// Currently only supports GitHub teams. Returns `None` for other providers
    /// or invalid login formats.
    pub fn url(&self) -> Option<String> {
        let (provider, org, _team) = self.split_login()?;
        match provider {
            "github" => Some(format!("https://github.com/{org}")),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_team(login: &str) -> Team {
        Team {
            id: 1,
            login: login.to_string(),
            github_id: 1000,
            name: None,
            avatar: None,
            org_id: 2000,
        }
    }

    mod split_login {
        use super::*;

        #[test]
        fn valid_login() {
            let team = new_team("github:rust-lang:core");
            assert_eq!(team.split_login(), Some(("github", "rust-lang", "core")));
        }

        #[test]
        fn missing_second_colon() {
            let team = new_team("github:rust-lang");
            assert_eq!(team.split_login(), None);
        }

        #[test]
        fn missing_both_colons() {
            let team = new_team("github");
            assert_eq!(team.split_login(), None);
        }

        #[test]
        fn empty_string() {
            let team = new_team("");
            assert_eq!(team.split_login(), None);
        }

        #[test]
        fn extra_colons() {
            let team = new_team("github:rust-lang:core:extra");
            assert_eq!(
                team.split_login(),
                Some(("github", "rust-lang", "core:extra"))
            );
        }

        #[test]
        fn different_provider() {
            let team = new_team("gitlab:my-org:my-team");
            assert_eq!(team.split_login(), Some(("gitlab", "my-org", "my-team")));
        }
    }

    mod url {
        use super::*;

        #[test]
        fn github_team() {
            let team = new_team("github:rust-lang:core");
            assert_eq!(team.url(), Some("https://github.com/rust-lang".to_string()));
        }

        #[test]
        fn gitlab_team_returns_none() {
            let team = new_team("gitlab:my-org:my-team");
            assert_eq!(team.url(), None);
        }

        #[test]
        fn invalid_format_returns_none() {
            let team = new_team("github:rust-lang");
            assert_eq!(team.url(), None);
        }

        #[test]
        fn empty_login_returns_none() {
            let team = new_team("");
            assert_eq!(team.url(), None);
        }

        #[test]
        fn github_with_hyphenated_org() {
            let team = new_team("github:test-org:core");
            assert_eq!(team.url(), Some("https://github.com/test-org".to_string()));
        }
    }
}
