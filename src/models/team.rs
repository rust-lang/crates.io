use bon::Builder;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use http::StatusCode;

use crate::util::errors::{bad_request, custom, AppResult};

use crates_io_github::{GitHubClient, GitHubError};
use oauth2::AccessToken;

use crate::models::{Crate, CrateOwner, Owner, OwnerKind, User};
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
    pub org_id: Option<i32>,
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
    /// Tries to create the Team in the DB (assumes a `:` has already been found).
    ///
    /// # Panics
    ///
    /// This function will panic if login contains less than 2 `:` characters.
    pub async fn create_or_update(
        gh_client: &dyn GitHubClient,
        conn: &mut AsyncPgConnection,
        login: &str,
        req_user: &User,
    ) -> AppResult<Self> {
        // must look like system:xxxxxxx
        let mut chunks = login.split(':');
        // unwrap is okay, split on an empty string still has 1 chunk
        match chunks.next().unwrap() {
            // github:rust-lang:owners
            "github" => {
                // unwrap is documented above as part of the calling contract
                let org = chunks.next().unwrap();
                let team = chunks.next().ok_or_else(|| {
                    bad_request(
                        "missing github team argument; \
                         format is github:org:team",
                    )
                })?;
                Team::create_or_update_github_team(
                    gh_client,
                    conn,
                    &login.to_lowercase(),
                    org,
                    team,
                    req_user,
                )
                .await
            }
            _ => Err(bad_request(
                "unknown organization handler, \
                 only 'github:org:team' is supported",
            )),
        }
    }

    /// Tries to create or update a Github Team. Assumes `org` and `team` are
    /// correctly parsed out of the full `name`. `name` is passed as a
    /// convenience to avoid rebuilding it.
    async fn create_or_update_github_team(
        gh_client: &dyn GitHubClient,
        conn: &mut AsyncPgConnection,
        login: &str,
        org_name: &str,
        team_name: &str,
        req_user: &User,
    ) -> AppResult<Self> {
        // GET orgs/:org/teams
        // check that `team` is the `slug` in results, and grab its data

        // "sanitization"
        fn is_allowed_char(c: char) -> bool {
            matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_')
        }

        if let Some(c) = org_name.chars().find(|c| !is_allowed_char(*c)) {
            return Err(bad_request(format_args!(
                "organization cannot contain special \
                 characters like {c}"
            )));
        }

        let token = AccessToken::new(req_user.gh_access_token.clone());
        let team = gh_client.team_by_name(org_name, team_name, &token).await
            .map_err(|_| {
                bad_request(format_args!(
                    "could not find the github team {org_name}/{team_name}. \
                    Make sure that you have the right permissions in GitHub. \
                    See https://doc.rust-lang.org/cargo/reference/publishing.html#github-permissions"
                ))
            })?;

        let org_id = team.organization.id;

        if !can_add_team(gh_client, org_id, team.id, req_user).await? {
            return Err(custom(
                StatusCode::FORBIDDEN,
                "only members of a team or organization owners can add it as an owner",
            ));
        }

        let org = gh_client.org_by_name(org_name, &token).await?;

        NewTeam::builder()
            .login(&login.to_lowercase())
            .org_id(org_id)
            .github_id(team.id)
            .maybe_name(team.name.as_deref())
            .maybe_avatar(org.avatar_url.as_deref())
            .build()
            .create_or_update(conn)
            .await
            .map_err(Into::into)
    }

    /// Phones home to Github to ask if this User is a member of the given team.
    /// Note that we're assuming that the given user is the one interested in
    /// the answer. If this is not the case, then we could accidentally leak
    /// private membership information here.
    pub async fn contains_user(
        &self,
        gh_client: &dyn GitHubClient,
        user: &User,
    ) -> AppResult<bool> {
        match self.org_id {
            Some(org_id) => {
                team_with_gh_id_contains_user(gh_client, org_id, self.github_id, user).await
            }
            // This means we don't have an org_id on file for the `self` team. It much
            // probably was deleted from github by the time we backfilled the database.
            // Short-circuiting to false since a non-existent team cannot contain any
            // user
            None => Ok(false),
        }
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

async fn can_add_team(
    gh_client: &dyn GitHubClient,
    org_id: i32,
    team_id: i32,
    user: &User,
) -> AppResult<bool> {
    Ok(
        team_with_gh_id_contains_user(gh_client, org_id, team_id, user).await?
            || is_gh_org_owner(gh_client, org_id, user).await?,
    )
}

async fn is_gh_org_owner(
    gh_client: &dyn GitHubClient,
    org_id: i32,
    user: &User,
) -> AppResult<bool> {
    let token = AccessToken::new(user.gh_access_token.clone());
    match gh_client
        .org_membership(org_id, &user.gh_login, &token)
        .await
    {
        Ok(membership) => Ok(membership.state == "active" && membership.role == "admin"),
        Err(GitHubError::NotFound(_)) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

async fn team_with_gh_id_contains_user(
    gh_client: &dyn GitHubClient,
    github_org_id: i32,
    github_team_id: i32,
    user: &User,
) -> AppResult<bool> {
    // GET /organizations/:org_id/team/:team_id/memberships/:username
    // check that "state": "active"

    let token = AccessToken::new(user.gh_access_token.clone());
    let membership = match gh_client
        .team_membership(github_org_id, github_team_id, &user.gh_login, &token)
        .await
    {
        // Officially how `false` is returned
        Err(GitHubError::NotFound(_)) => return Ok(false),
        x => x?,
    };

    // There is also `state: pending` for which we could possibly give
    // some feedback, but it's not obvious how that should work.
    Ok(membership.state == "active")
}
