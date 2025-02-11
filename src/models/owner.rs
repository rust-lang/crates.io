use diesel::pg::Pg;
use diesel::prelude::*;

use crate::models::{Crate, CrateOwnerInvitation, Team, User};
use crate::schema::crate_owners;
use crates_io_diesel_helpers::pg_enum;

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

    pub fn from_invite(invite: &CrateOwnerInvitation) -> Self {
        Self {
            crate_id: invite.crate_id,
            owner_id: invite.invited_user_id,
            created_by: invite.invited_by_user_id,
            owner_kind: OwnerKind::User,
            email_notifications: true,
        }
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
