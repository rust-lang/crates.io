use diesel::pg::Pg;
use diesel::prelude::*;

use crate::app::App;
use crate::util::errors::{bad_request, AppResult};

use crate::models::{Crate, Team, User};
use crate::schema::crate_owners;
use crate::sql::pg_enum;
use crate::util::diesel::Conn;

#[derive(Insertable, Associations, Identifiable, Debug, Clone, Copy)]
#[diesel(
    table_name = crate_owners,
    check_for_backend(diesel::pg::Pg),
    primary_key(crate_id, owner_id, owner_kind),
    belongs_to(Crate),
    belongs_to(User, foreign_key = owner_id),
    belongs_to(Team, foreign_key = owner_id),
)]
pub struct CrateOwner {
    pub crate_id: i32,
    pub owner_id: i32,
    pub created_by: i32,
    pub owner_kind: OwnerKind,
    pub email_notifications: bool,
}

type BoxedQuery<'a> = crate_owners::BoxedQuery<'a, Pg, crate_owners::SqlType>;

impl CrateOwner {
    /// Returns a base crate owner query filtered by the owner kind argument. This query also
    /// filters out deleted records.
    pub fn by_owner_kind(kind: OwnerKind) -> BoxedQuery<'static> {
        crate_owners::table
            .filter(crate_owners::deleted.eq(false))
            .filter(crate_owners::owner_kind.eq(kind))
            .into_boxed()
    }
}

pg_enum! {
    pub enum OwnerKind {
        User = 0,
        Team = 1,
    }
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
    ///
    /// May be a user's GH login or a full team name. This is case
    /// sensitive.
    pub fn find_or_create_by_login(
        app: &App,
        conn: &mut impl Conn,
        req_user: &User,
        name: &str,
    ) -> AppResult<Owner> {
        if name.contains(':') {
            Ok(Owner::Team(Team::create_or_update(
                app, conn, name, req_user,
            )?))
        } else {
            User::find_by_login(conn, name)
                .optional()?
                .map(Owner::User)
                .ok_or_else(|| bad_request(format_args!("could not find user with login `{name}`")))
        }
    }

    pub fn kind(&self) -> i32 {
        match self {
            Owner::User(_) => OwnerKind::User as i32,
            Owner::Team(_) => OwnerKind::Team as i32,
        }
    }

    pub fn login(&self) -> &str {
        match self {
            Owner::User(user) => &user.gh_login,
            Owner::Team(team) => &team.login,
        }
    }

    pub fn id(&self) -> i32 {
        match self {
            Owner::User(user) => user.id,
            Owner::Team(team) => team.id,
        }
    }
}
