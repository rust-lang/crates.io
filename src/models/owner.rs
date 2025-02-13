use bon::Builder;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use self::crate_owner_builder::{SetOwnerId, SetOwnerKind};
use crate::models::{Crate, CrateOwnerInvitation, Team, User};
use crate::schema::crate_owners;
use crates_io_diesel_helpers::pg_enum;

#[derive(Insertable, Associations, Identifiable, Debug, Clone, Copy, Builder)]
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
    #[builder(setters(vis = "pub(self)"))]
    pub owner_id: i32,
    pub created_by: i32,
    #[builder(setters(vis = "pub(self)"))]
    pub owner_kind: OwnerKind,
    #[builder(default = true)]
    pub email_notifications: bool,
}

impl<S: crate_owner_builder::State> CrateOwnerBuilder<S> {
    pub fn team_id(self, team_id: i32) -> CrateOwnerBuilder<SetOwnerId<SetOwnerKind<S>>>
    where
        S::OwnerId: crate_owner_builder::IsUnset,
        S::OwnerKind: crate_owner_builder::IsUnset,
    {
        self.owner_kind(OwnerKind::Team).owner_id(team_id)
    }

    pub fn user_id(self, user_id: i32) -> CrateOwnerBuilder<SetOwnerId<SetOwnerKind<S>>>
    where
        S::OwnerId: crate_owner_builder::IsUnset,
        S::OwnerKind: crate_owner_builder::IsUnset,
    {
        self.owner_kind(OwnerKind::User).owner_id(user_id)
    }
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
        CrateOwner::builder()
            .crate_id(invite.crate_id)
            .user_id(invite.invited_user_id)
            .created_by(invite.invited_by_user_id)
            .build()
    }

    /// Inserts the crate owner into the database, or removes the `deleted` flag
    /// if the record already exists.
    pub async fn insert(&self, conn: &mut AsyncPgConnection) -> QueryResult<()> {
        diesel::insert_into(crate_owners::table)
            .values(self)
            .on_conflict(crate_owners::table.primary_key())
            .do_update()
            .set(crate_owners::deleted.eq(false))
            .execute(conn)
            .await
            .map(|_| ())
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
