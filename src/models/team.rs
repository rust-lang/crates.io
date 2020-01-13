use diesel::prelude::*;

use crate::app::App;
use crate::github::{github_api, team_url};
use crate::util::errors::{cargo_err, AppResult, NotFound};

use oauth2::{prelude::*, AccessToken};

use crate::models::{Crate, CrateOwner, Owner, OwnerKind, User};
use crate::schema::{crate_owners, teams};
use crate::views::EncodableTeam;

/// For now, just a Github Team. Can be upgraded to other teams
/// later if desirable.
#[derive(Queryable, Identifiable, Serialize, Deserialize, Debug)]
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
}

#[derive(Insertable, AsChangeset, Debug)]
#[table_name = "teams"]
pub struct NewTeam<'a> {
    pub login: &'a str,
    pub github_id: i32,
    pub name: Option<String>,
    pub avatar: Option<String>,
}

impl<'a> NewTeam<'a> {
    pub fn new(
        login: &'a str,
        github_id: i32,
        name: Option<String>,
        avatar: Option<String>,
    ) -> Self {
        NewTeam {
            login,
            github_id,
            name,
            avatar,
        }
    }

    pub fn create_or_update(&self, conn: &PgConnection) -> QueryResult<Team> {
        use crate::schema::teams::dsl::*;
        use diesel::insert_into;

        insert_into(teams)
            .values(self)
            .on_conflict(login)
            .do_update()
            .set(self)
            .get_result(conn)
    }
}

impl Team {
    /// Tries to create the Team in the DB (assumes a `:` has already been found).
    ///
    /// # Panics
    ///
    /// This function will panic if login contains less than 2 `:` characters.
    pub fn create_or_update(
        app: &App,
        conn: &PgConnection,
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
                    cargo_err(
                        "missing github team argument; \
                         format is github:org:team",
                    )
                })?;
                Team::create_or_update_github_team(
                    app,
                    conn,
                    &login.to_lowercase(),
                    org,
                    team,
                    req_user,
                )
            }
            _ => Err(cargo_err(
                "unknown organization handler, \
                 only 'github:org:team' is supported",
            )),
        }
    }

    /// Tries to create or update a Github Team. Assumes `org` and `team` are
    /// correctly parsed out of the full `name`. `name` is passed as a
    /// convenience to avoid rebuilding it.
    fn create_or_update_github_team(
        app: &App,
        conn: &PgConnection,
        login: &str,
        org_name: &str,
        team_name: &str,
        req_user: &User,
    ) -> AppResult<Self> {
        // GET orgs/:org/teams
        // check that `team` is the `slug` in results, and grab its data

        // "sanitization"
        fn whitelist(c: char) -> bool {
            match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => false,
                _ => true,
            }
        }

        if let Some(c) = org_name.chars().find(|c| whitelist(*c)) {
            return Err(cargo_err(&format_args!(
                "organization cannot contain special \
                 characters like {}",
                c
            )));
        }

        #[derive(Deserialize)]
        struct GithubTeam {
            id: i32,              // unique GH id (needed for membership queries)
            name: Option<String>, // Pretty name
        }

        let url = format!("/orgs/{}/teams/{}", org_name, team_name);
        let token = AccessToken::new(req_user.gh_access_token.clone());
        let team = github_api::<GithubTeam>(app, &url, &token).map_err(|_| {
            cargo_err(&format_args!(
                "could not find the github team {}/{}",
                org_name, team_name
            ))
        })?;

        if !team_with_gh_id_contains_user(app, team.id, req_user)? {
            return Err(cargo_err("only members of a team can add it as an owner"));
        }

        #[derive(Deserialize)]
        struct Org {
            avatar_url: Option<String>,
        }

        let url = format!("/orgs/{}", org_name);
        let org = github_api::<Org>(app, &url, &token)?;

        NewTeam::new(&login.to_lowercase(), team.id, team.name, org.avatar_url)
            .create_or_update(conn)
            .map_err(Into::into)
    }

    /// Phones home to Github to ask if this User is a member of the given team.
    /// Note that we're assuming that the given user is the one interested in
    /// the answer. If this is not the case, then we could accidentally leak
    /// private membership information here.
    pub fn contains_user(&self, app: &App, user: &User) -> AppResult<bool> {
        team_with_gh_id_contains_user(app, self.github_id, user)
    }

    pub fn owning(krate: &Crate, conn: &PgConnection) -> QueryResult<Vec<Owner>> {
        let base_query = CrateOwner::belonging_to(krate).filter(crate_owners::deleted.eq(false));
        let teams = base_query
            .inner_join(teams::table)
            .select(teams::all_columns)
            .filter(crate_owners::owner_kind.eq(OwnerKind::Team as i32))
            .load(conn)?
            .into_iter()
            .map(Owner::Team);

        Ok(teams.collect())
    }

    pub fn encodable(self) -> EncodableTeam {
        let Team {
            id,
            name,
            login,
            avatar,
            ..
        } = self;
        let url = team_url(&login);

        EncodableTeam {
            id,
            login,
            name,
            avatar,
            url: Some(url),
        }
    }
}

fn team_with_gh_id_contains_user(app: &App, github_id: i32, user: &User) -> AppResult<bool> {
    // GET teams/:team_id/memberships/:user_name
    // check that "state": "active"

    #[derive(Deserialize)]
    struct Membership {
        state: String,
    }

    let url = format!("/teams/{}/memberships/{}", &github_id, &user.gh_login);
    let token = AccessToken::new(user.gh_access_token.clone());
    let membership = match github_api::<Membership>(app, &url, &token) {
        // Officially how `false` is returned
        Err(ref e) if e.is::<NotFound>() => return Ok(false),
        x => x?,
    };

    // There is also `state: pending` for which we could possibly give
    // some feedback, but it's not obvious how that should work.
    Ok(membership.state == "active")
}
