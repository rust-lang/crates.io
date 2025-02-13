use bon::Builder;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crates_io_github::{GitHubClient, GitHubError};
use oauth2::AccessToken;

use crate::models::{Crate, CrateOwner, Owner, OwnerKind};
use crate::schema::{crate_owners, teams};

/// For now, just a Github Team. Can be upgraded to other teams
/// later if desirable.
#[derive(Queryable, Identifiable, Serialize, Deserialize, Debug, Selectable)]
pub struct Team {
    /// Unique table id
    pub id: i32,
    /// "github:org:team"
    /// An opaque unique ID, that was at one point parsed out to query Github.
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
            .get_result(conn)
            .await
    }
}

impl Team {
    /// Phones home to Github to ask if this User is a member of the given team.
    /// Note that we're assuming that the given user is the one interested in
    /// the answer. If this is not the case, then we could accidentally leak
    /// private membership information here.
    pub async fn contains_user(
        &self,
        gh_client: &dyn GitHubClient,
        gh_login: &str,
        token: &AccessToken,
    ) -> Result<bool, GitHubError> {
        team_with_gh_id_contains_user(gh_client, self.org_id, self.github_id, gh_login, token).await
    }

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
}

pub async fn can_add_team(
    gh_client: &dyn GitHubClient,
    org_id: i32,
    team_id: i32,
    gh_login: &str,
    token: &AccessToken,
) -> Result<bool, GitHubError> {
    Ok(
        team_with_gh_id_contains_user(gh_client, org_id, team_id, gh_login, token).await?
            || is_gh_org_owner(gh_client, org_id, gh_login, token).await?,
    )
}

async fn is_gh_org_owner(
    gh_client: &dyn GitHubClient,
    org_id: i32,
    gh_login: &str,
    token: &AccessToken,
) -> Result<bool, GitHubError> {
    let membership = gh_client.org_membership(org_id, gh_login, token).await?;
    Ok(membership.is_some_and(|m| m.is_active_admin()))
}

async fn team_with_gh_id_contains_user(
    gh_client: &dyn GitHubClient,
    github_org_id: i32,
    github_team_id: i32,
    gh_login: &str,
    token: &AccessToken,
) -> Result<bool, GitHubError> {
    // GET /organizations/:org_id/team/:team_id/memberships/:username
    // check that "state": "active"

    let membership = gh_client
        .team_membership(github_org_id, github_team_id, gh_login, token)
        .await?;

    // There is also `state: pending` for which we could possibly give
    // some feedback, but it's not obvious how that should work.
    Ok(membership.is_some_and(|m| m.is_active()))
}
