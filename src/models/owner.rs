use diesel::pg::Pg;
use diesel::prelude::*;

use crate::util::errors::{cargo_err, AppResult};
use crate::{app::App, schema::teams};

use crate::models::{Crate, Team, User};
use crate::schema::{crate_owners, users};
use crate::sql::{lower, pg_enum};

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
        use self::crate_owners::dsl::*;

        crate_owners
            .filter(deleted.eq(false))
            .filter(owner_kind.eq(kind))
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
        conn: &mut PgConnection,
        req_user: &User,
        name: &str,
    ) -> AppResult<Owner> {
        if name.contains(':') {
            Ok(Owner::Team(Team::create_or_update(
                app, conn, name, req_user,
            )?))
        } else {
            users::table
                .filter(lower(users::gh_login).eq(name.to_lowercase()))
                .filter(users::gh_id.ne(-1))
                .order(users::gh_id.desc())
                .first(conn)
                .map(Owner::User)
                .map_err(|_| cargo_err(&format_args!("could not find user with login `{name}`")))
        }
    }

    /// Finds the owner by name. Never recreates a team, to ensure that
    /// organizations that were deleted after they were added can still be
    /// removed.
    ///
    /// May be a user's GH login or a full team name. This is case
    /// sensitive.
    pub fn find_by_login(conn: &mut PgConnection, name: &str) -> AppResult<Owner> {
        if name.contains(':') {
            teams::table
                .filter(lower(teams::login).eq(&name.to_lowercase()))
                .first(conn)
                .map(Owner::Team)
                .map_err(|_| cargo_err(&format_args!("could not find team with login `{name}`")))
        } else {
            users::table
                .filter(lower(users::gh_login).eq(name.to_lowercase()))
                .filter(users::gh_id.ne(-1))
                .order(users::gh_id.desc())
                .first(conn)
                .map(Owner::User)
                .map_err(|_| cargo_err(&format_args!("could not find user with login `{name}`")))
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
}
