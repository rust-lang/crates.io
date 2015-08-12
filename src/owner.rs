use {Model, User};
use util::{RequestUtils, CargoResult, internal, ChainError, human};
use db::Connection;
use pg::rows::Row;
use rustc_serialize::json;
use util::errors::NotFound;
use std::str;
use http;
use app::App;

#[repr(u32)]
pub enum OwnerKind {
    User = 0,
    Team = 1,
}

/// Unifies the notion of a User or a Team.
pub enum Owner {
    User(User),
    Team(Team),
}

/// For now, just a Github Team. Can be upgraded to other teams
/// later if desirable.
pub struct Team {
    /// We're assuming these are stable
    pub github_id: i32,
    /// Unique table id
    pub id: i32,
    /// "github:org:team"
    /// An opaque unique ID, that was at one point parsed out to query Github.
    /// We only query membership with github using the github_id, though.
    pub name: String,
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct EncodableOwner {
    pub id: i32,
    // TODO: duplicate this field with better name
    pub login: String,
    pub kind: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub avatar: Option<String>,
}

/// Access rights to the crate (publishing and ownership management)
/// NOTE: The order of these variants matters!
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum Rights {
    None,
    Publish,
    Full,
}

impl Team {
    /// Just gets the Team from the database by name.
    pub fn find_by_name(conn: &Connection, name: &str) -> CargoResult<Self> {
        let stmt = try!(conn.prepare("SELECT * FROM teams
                                      WHERE name = $1"));
        let rows = try!(stmt.query(&[&name]));
        let row = try!(rows.iter().next().chain_error(|| {
            NotFound
        }));
        Ok(Model::from_row(&row))
    }

    /// Tries to create the Team in the DB (assumes a `:` has already been found).
    pub fn create(app: &App, conn: &Connection, name: &str, req_user: &User) -> CargoResult<Self> {
        // must look like system:xxxxxxx
        let mut chunks = name.split(":");
        match chunks.next().unwrap() {
            // github:rust-lang:owners
            "github" => {
                // Ok to unwrap since we know one ":" is contained
                let org = chunks.next().unwrap();
                let team = try!(chunks.next().ok_or_else(||
                    human("missing github team argument; format is github:org:team")
                ));
                Team::create_github_team(app, conn, name, org, team, req_user)
            }
            _ => {
                Err(human("unknown organization handler, only 'github:org:team' is supported"))
            }
        }
    }

    /// Tries to create a Github Team from scratch. Assumes `org` and `team` are
    /// correctly parsed out of the full `name`. `name` is passed as a convenience
    /// to avoid rebuilding it.
    pub fn create_github_team(app: &App, conn: &Connection, name: &str, org_name: &str,
                                team_name: &str, req_user: &User) -> CargoResult<Self> {
        // GET orgs/:org/teams
        // check that `team` is the `slug` in results, and grab its `id`

        // "sanitization"
        fn whitelist(c: &char) -> bool {
            match *c {
                'a'...'z' | 'A'...'Z' | '0'...'9' | '-' | '_' => false,
                _ => true
            }
        }

        if let Some(c) = org_name.chars().find(whitelist) {
            return Err(human(format!("organization cannot contain special characters like {}",
                                   c)));
        }

        let resp = try!(http::github(app,
            &format!("http://api.github.com/orgs/{}/teams", org_name),
            &http::token(req_user.gh_access_token.clone())));

        match resp.get_code() {
            200 => {} // Ok!
            403 => {
                return Err(human("It looks like you don't have permission \
                                  to query one of these organizations. \
                                  You may need to re-authenticate on \
                                  crates.io to grant permission to read \
                                  github org memberships. Just go to \
                                  https://crates.io/login"));
            }
            _ => {
                return Err(internal(format!("didn't get a 200 result from github: {}",
                                            resp)));
            }
        }

        #[derive(RustcDecodable)]
        struct GithubTeam {
            slug: String,
            id: i32,
        }

        let json = try!(str::from_utf8(resp.get_body()).ok().chain_error(||{
            internal("github didn't send a utf8-response")
        }));
        let teams: Vec<GithubTeam> = try!(json::decode(json).chain_error(|| {
            internal("github didn't send a valid json response")
        }));

        let mut github_id = None;

        for team in teams {
            if team.slug == team_name {
                github_id = Some(team.id);
                break;
            }
        }

        let github_id = try!(github_id.ok_or_else(|| {
            human(format!("could not find the github team {}/{}", org_name, team_name))
        }));

        if !try!(team_with_gh_id_contains_user(app, github_id, req_user)) {
            return Err(human("only members of a team can add it as an owner"));
        }

        Team::insert(conn, name, github_id)
    }

    pub fn insert(conn: &Connection, name: &str, github_id: i32) -> CargoResult<Self> {
        // insert into DB for reals
        let stmt = try!(conn.prepare("INSERT INTO teams
                                   (name, github_id)
                                   VALUES ($1, $2)
                                   RETURNING *"));

        let rows = try!(stmt.query(&[&name, &github_id]));
        let row = rows.iter().next().unwrap();
        Ok(Model::from_row(&row))
    }

    /// Phones home to Github to ask if this User is a member of the given team.
    /// Note that we're assuming that the given user is the one interested in
    /// the answer. If this is not the case, then we could accidentally leak
    /// private membership information here.
    pub fn contains_user(&self, app: &App, user: &User) -> CargoResult<bool> {
        team_with_gh_id_contains_user(app, self.github_id, user)
    }
}

fn team_with_gh_id_contains_user(app: &App, github_id: i32, user: &User) -> CargoResult<bool> {
    // GET teams/:team_id/memberships/:user_name
    // check that "state": "active"

    let resp = try!(http::github(app,
        &format!("http://api.github.com/teams/{}/memberships/{}", &github_id, &user.gh_login),
        &http::token(user.gh_access_token.clone())));

    match resp.get_code() {
        200 => {} // Ok!
        404 => {
            // Yes, this is actually how "no membership" is signaled
            return Ok(false);
        }
        403 => {
            return Err(human("It looks like you don't have permission \
                              to query an organization that owns this \
                              crate. You may need to re-authenticate on \
                              crates.io to grant permission to read \
                              github org memberships. Just go to \
                              https://crates.io/login"));
        }
        _ => {
            return Err(internal(format!("didn't get a 200 result from github: {}",
                                    resp)))
        }
    }

    #[derive(RustcDecodable)]
    struct Membership {
        state: String,
    }
    let json = try!(str::from_utf8(resp.get_body()).ok().chain_error(||{
        internal("github didn't send a utf8-response")
    }));
    let membership: Membership = try!(json::decode(json).chain_error(|| {
        internal("github didn't send a valid json response")
    }));

    // There is also `state: pending` for which we could possibly give
    // some feedback, but it's not obvious how that should work.
    Ok(membership.state == "active")
}

impl Model for Team {
    fn from_row(row: &Row) -> Self {
        Team {
            id: row.get("id"),
            name: row.get("name"),
            github_id: row.get("github_id"),
        }
    }

    fn table_name(_: Option<Self>) -> &'static str { "teams" }
}

impl Owner {
    /// Finds the owner by name, failing out if it doesn't exist.
    /// May be a user's GH login, or a full team name. This is case
    /// sensitive.
    pub fn find_by_name(conn: &Connection, name: &str) -> CargoResult<Owner> {
        let owner = if name.contains(":") {
            Owner::Team(try!(Team::find_by_name(conn, name).map_err(|_|
                human(format!("could not find team with name {}", name))
            )))
        } else {
            Owner::User(try!(User::find_by_login(conn, name).map_err(|_|
                human(format!("could not find user with login `{}`", name))
            )))
        };
        Ok(owner)
    }

    /// Find the owner by name, with the intent of adding it as an owner.
    ///
    /// This differs from find_by_name in that in the case of a Team,
    /// it will verify the req_user is on the team first.
    ///
    /// If the req_user is on the Team, it will create the team in the DB
    /// if it is not already present. When this occurs, this will set the
    /// One True Name of the team. All future references to the team must
    /// use the exact casing provided here. If a different casing is provided
    /// to this method, we may still succeed if Github returns us the same ID
    /// as the One True Name. However in this case, the One True Name will
    /// still be selected.
    pub fn find_by_name_for_add(app: &App, conn: &Connection, name: &str, req_user: &User)
        -> CargoResult<Owner> {
        if !name.contains(":") {
            return Ok(Owner::User(try!(User::find_by_login(conn, name).map_err(|_|
                human(format!("could not find user with login `{}`", name))
            ))));
        }

        // We're working with a Team, try to just get it out of the DB.
        if let Ok(team) = Team::find_by_name(conn, name) {
            return if try!(team.contains_user(app, req_user)) {
                Ok(Owner::Team(team))
            } else {
                Err(human(format!("only members of {} can add it as an owner", name)))
            };
        }

        // Failed to retrieve from the DB, must be a new Team, try to add it.
        Ok(Owner::Team(try!(Team::create(app, conn, name, req_user))))
    }

    pub fn kind(&self) -> i32 {
        match *self {
            Owner::User(_) => OwnerKind::User as i32,
            Owner::Team(_) => OwnerKind::Team as i32,
        }
    }

    pub fn name(&self) -> &str {
        match *self {
            Owner::User(ref user) => &user.gh_login,
            Owner::Team(ref team) => &team.name,
        }
    }

    pub fn id(&self) -> i32 {
        match *self {
            Owner::User(ref user) => user.id,
            Owner::Team(ref team) => team.id,
        }
    }

    pub fn encodable(self) -> EncodableOwner {
        match self {
            Owner::User(User { id, email, name, gh_login, avatar, .. }) => {
                EncodableOwner {
                    id: id,
                    login: gh_login,
                    email: email,
                    avatar: avatar,
                    name: name,
                    kind: String::from("user"),
                }
            }
            Owner::Team(Team { id, name, .. }) => {
                EncodableOwner {
                    id: id,
                    login: name,
                    email: None,
                    avatar: None,
                    name: None,
                    kind: String::from("team"),
                }
            }
        }
    }
}

/// Given this set of owners, determines the strongest rights the
/// given user has.
///
/// Shortcircuits on `Full` because you can't beat it. In practice we'll always
/// see `[user, user, user, ..., team, team, team]`, so we could shortcircuit on
/// `Publish` as well, but this is a non-obvious invariant so we don't bother.
/// Sweet free optimization if teams are proving burdensome to check.
/// More than one team isn't really expected, though.
pub fn rights(app: &App, owners: &[Owner], user: &User) -> CargoResult<Rights> {
    let mut best = Rights::None;
    for owner in owners {
        match *owner {
            Owner::User(ref other_user) => if other_user.id == user.id {
                return Ok(Rights::Full);
            },
            Owner::Team(ref team) => if try!(team.contains_user(app, user)) {
                best = Rights::Publish;
            },
        }
    }
    Ok(best)
}

