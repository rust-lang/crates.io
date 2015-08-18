use {Model, User};
use util::{RequestUtils, CargoResult, ChainError, human};
use db::Connection;
use pg::rows::Row;
use util::errors::NotFound;
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
    /// This is the only name we should ever talk to Cargo about.
    pub login: String,
    /// Sugary goodness
    pub name: Option<String>,
    pub avatar: Option<String>,

}

#[derive(RustcEncodable)]
pub struct EncodableOwner {
    pub id: i32,
    pub login: String,
    pub kind: String,
    pub email: Option<String>,
    pub url: Option<String>,
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
    pub fn find_by_login(conn: &Connection, login: &str) -> CargoResult<Self> {
        let stmt = try!(conn.prepare("SELECT * FROM teams
                                      WHERE login = $1"));
        let rows = try!(stmt.query(&[&login]));
        let row = try!(rows.iter().next().chain_error(|| {
            NotFound
        }));
        Ok(Model::from_row(&row))
    }

    /// Tries to create the Team in the DB (assumes a `:` has already been found).
    pub fn create(app: &App, conn: &Connection, login: &str, req_user: &User)
                                                    -> CargoResult<Self> {
        // must look like system:xxxxxxx
        let mut chunks = login.split(":");
        match chunks.next().unwrap() {
            // github:rust-lang:owners
            "github" => {
                // Ok to unwrap since we know one ":" is contained
                let org = chunks.next().unwrap();
                let team = try!(chunks.next().ok_or_else(||
                    human("missing github team argument; \
                            format is github:org:team")
                ));
                Team::create_github_team(app, conn, login, org, team, req_user)
            }
            _ => {
                Err(human("unknown organization handler, \
                            only 'github:org:team' is supported"))
            }
        }
    }

    /// Tries to create a Github Team from scratch. Assumes `org` and `team` are
    /// correctly parsed out of the full `name`. `name` is passed as a
    /// convenience to avoid rebuilding it.
    pub fn create_github_team(app: &App, conn: &Connection, login: &str,
                              org_name: &str, team_name: &str, req_user: &User)
                              -> CargoResult<Self> {
        // GET orgs/:org/teams
        // check that `team` is the `slug` in results, and grab its data

        // "sanitization"
        fn whitelist(c: &char) -> bool {
            match *c {
                'a'...'z' | 'A'...'Z' | '0'...'9' | '-' | '_' => false,
                _ => true
            }
        }

        if let Some(c) = org_name.chars().find(whitelist) {
            return Err(human(format!("organization cannot contain special \
                                        characters like {}", c)));
        }

        #[derive(RustcDecodable)]
        struct GithubTeam {
            slug: String,   // the name we want to find
            id: i32,        // unique GH id (needed for membership queries)
            name: Option<String>,   // Pretty name
        }

        // FIXME: we just set per_page=100 and don't bother chasing pagination
        // links. A hundred teams should be enough for any org, right?
        let url = format!("/orgs/{}/teams", org_name);
        let token = http::token(req_user.gh_access_token.clone());
        let resp = try!(http::github(app, &url, &token));
        let teams: Vec<GithubTeam> = try!(http::parse_github_response(resp));

        let team = try!(teams.into_iter().find(|team| team.slug == team_name)
            .ok_or_else(||{
                human(format!("could not find the github team {}/{}",
                            org_name, team_name))
            })
        );

        if !try!(team_with_gh_id_contains_user(app, team.id, req_user)) {
            return Err(human("only members of a team can add it as an owner"));
        }

        #[derive(RustcDecodable)]
        struct Org {
            avatar_url: Option<String>,
        }

        let url = format!("/orgs/{}", org_name);
        let resp = try!(http::github(app, &url, &token));
        let org: Org = try!(http::parse_github_response(resp));

        Team::insert(conn, login, team.id, team.name, org.avatar_url)
    }

    pub fn insert(conn: &Connection,
                  login: &str,
                  github_id: i32,
                  name: Option<String>,
                  avatar: Option<String>)
                  -> CargoResult<Self> {

        let stmt = try!(conn.prepare("INSERT INTO teams
                                   (login, github_id, name, avatar)
                                   VALUES ($1, $2, $3, $4)
                                   RETURNING *"));

        let rows = try!(stmt.query(&[&login, &github_id, &name, &avatar]));
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

fn team_with_gh_id_contains_user(app: &App, github_id: i32, user: &User)
                                                -> CargoResult<bool> {
    // GET teams/:team_id/memberships/:user_name
    // check that "state": "active"

    #[derive(RustcDecodable)]
    struct Membership {
        state: String,
    }

    let url = format!("/teams/{}/memberships/{}",
                        &github_id, &user.gh_login);
    let token = http::token(user.gh_access_token.clone());
    let resp = try!(http::github(app, &url, &token));

    // Officially how `false` is returned
    if resp.get_code() == 404 { return Ok(false) }

    let membership: Membership = try!(http::parse_github_response(resp));

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
            login: row.get("login"),
            avatar: row.get("avatar"),
        }
    }

    fn table_name(_: Option<Self>) -> &'static str { "teams" }
}

impl Owner {
    /// Finds the owner by name, failing out if it doesn't exist.
    /// May be a user's GH login, or a full team name. This is case
    /// sensitive.
    pub fn find_by_login(conn: &Connection, name: &str) -> CargoResult<Owner> {
        let owner = if name.contains(":") {
            Owner::Team(try!(Team::find_by_login(conn, name).map_err(|_|
                human(format!("could not find team with name {}", name))
            )))
        } else {
            Owner::User(try!(User::find_by_login(conn, name).map_err(|_|
                human(format!("could not find user with login `{}`", name))
            )))
        };
        Ok(owner)
    }

    pub fn kind(&self) -> i32 {
        match *self {
            Owner::User(_) => OwnerKind::User as i32,
            Owner::Team(_) => OwnerKind::Team as i32,
        }
    }

    pub fn login(&self) -> &str {
        match *self {
            Owner::User(ref user) => &user.gh_login,
            Owner::Team(ref team) => &team.login,
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
                let url = format!("https://github.com/{}", gh_login);
                EncodableOwner {
                    id: id,
                    login: gh_login,
                    email: email,
                    avatar: avatar,
                    url: Some(url),
                    name: name,
                    kind: String::from("user"),
                }
            }
            Owner::Team(Team { id, name, login, avatar, .. }) => {
                let url = {
                    let mut parts = login.split(":");
                    parts.next(); // discard github
                    format!("https://github.com/{}/teams/{}",
                            parts.next().unwrap(), parts.next().unwrap())
                };
                EncodableOwner {
                    id: id,
                    login: login,
                    email: None,
                    url: Some(url),
                    avatar: avatar,
                    name: name,
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

