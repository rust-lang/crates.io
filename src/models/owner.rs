use diesel::prelude::*;

use crate::app::App;
use crate::github;
use crate::util::{human, CargoResult};

use crate::models::{Crate, Team, User};
use crate::schema::{crate_owners, users};
use crate::views::EncodableOwner;

#[derive(Insertable, Associations, Identifiable, Debug, Clone, Copy)]
#[belongs_to(Crate)]
#[belongs_to(User, foreign_key = "owner_id")]
#[belongs_to(Team, foreign_key = "owner_id")]
#[table_name = "crate_owners"]
#[primary_key(crate_id, owner_id, owner_kind)]
pub struct CrateOwner {
    pub crate_id: i32,
    pub owner_id: i32,
    pub created_by: i32,
    pub owner_kind: i32,
}

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum OwnerKind {
    User = 0,
    Team = 1,
}

/// Unifies the notion of a User or a Team.
#[derive(Debug)]
pub enum Owner {
    User(User),
    Team(Team),
}

impl Owner {
    /// Finds the owner by name. Always recreates teams to get the most
    /// up-to-date GitHub ID. Fails out if the user isn't found in the
    /// database, the team isn't found on GitHub, or if the user isn't a member
    /// of the team on GitHub.
    /// May be a user's GH login or a full team name. This is case
    /// sensitive.
    pub fn find_or_create_by_login(
        app: &App,
        conn: &PgConnection,
        req_user: &User,
        name: &str,
    ) -> CargoResult<Owner> {
        if name.contains(':') {
            Ok(Owner::Team(Team::create_or_update(
                app, conn, name, req_user,
            )?))
        } else {
            users::table
                .filter(users::gh_login.eq(name))
                .first(conn)
                .map(Owner::User)
                .map_err(|_| human(&format_args!("could not find user with login `{}`", name)))
        }
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
            Owner::User(User {
                id,
                name,
                gh_login,
                gh_avatar,
                ..
            }) => {
                let url = format!("https://github.com/{}", gh_login);
                EncodableOwner {
                    id,
                    login: gh_login,
                    avatar: gh_avatar,
                    url: Some(url),
                    name,
                    kind: String::from("user"),
                }
            }
            Owner::Team(Team {
                id,
                name,
                login,
                avatar,
                ..
            }) => {
                let url = github::team_url(&login);
                EncodableOwner {
                    id,
                    login,
                    url: Some(url),
                    avatar,
                    name,
                    kind: String::from("team"),
                }
            }
        }
    }
}
