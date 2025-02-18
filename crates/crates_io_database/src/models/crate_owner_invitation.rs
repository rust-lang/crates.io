use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use secrecy::SecretString;

use crate::models::CrateOwner;
use crate::schema::{crate_owner_invitations, crates};

#[derive(Debug)]
pub enum NewCrateOwnerInvitationOutcome {
    AlreadyExists,
    InviteCreated { plaintext_token: SecretString },
}

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = crate_owner_invitations, check_for_backend(diesel::pg::Pg))]
pub struct NewCrateOwnerInvitation {
    pub invited_user_id: i32,
    pub invited_by_user_id: i32,
    pub crate_id: i32,
    pub expires_at: DateTime<Utc>,
}

impl NewCrateOwnerInvitation {
    pub async fn create(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> QueryResult<NewCrateOwnerInvitationOutcome> {
        // Before actually creating the invite, check if an expired invitation already exists
        // and delete it from the database. This allows obtaining a new invite if the old one
        // expired, instead of returning "already exists".
        diesel::delete(crate_owner_invitations::table)
            .filter(crate_owner_invitations::invited_user_id.eq(self.invited_user_id))
            .filter(crate_owner_invitations::crate_id.eq(self.crate_id))
            .filter(crate_owner_invitations::expires_at.le(Utc::now()))
            .execute(conn)
            .await?;

        let res: Option<CrateOwnerInvitation> = diesel::insert_into(crate_owner_invitations::table)
            .values(self)
            // The ON CONFLICT DO NOTHING clause results in not creating the invite if another one
            // already exists. This does not cause problems with expired invitation as those are
            // deleted before doing this INSERT.
            .on_conflict_do_nothing()
            .get_result(conn)
            .await
            .optional()?;

        Ok(match res {
            Some(record) => NewCrateOwnerInvitationOutcome::InviteCreated {
                plaintext_token: record.token,
            },
            None => NewCrateOwnerInvitationOutcome::AlreadyExists,
        })
    }
}

/// The model representing a row in the `crate_owner_invitations` database table.
#[derive(Clone, Debug, Identifiable, Queryable)]
#[diesel(primary_key(invited_user_id, crate_id))]
pub struct CrateOwnerInvitation {
    pub invited_user_id: i32,
    pub invited_by_user_id: i32,
    pub crate_id: i32,
    pub created_at: DateTime<Utc>,
    #[diesel(deserialize_as = String)]
    pub token: SecretString,
    pub expires_at: DateTime<Utc>,
}

impl CrateOwnerInvitation {
    pub async fn find_by_id(
        user_id: i32,
        crate_id: i32,
        conn: &mut AsyncPgConnection,
    ) -> QueryResult<Self> {
        crate_owner_invitations::table
            .find((user_id, crate_id))
            .first::<Self>(conn)
            .await
    }

    pub async fn find_by_token(token: &str, conn: &mut AsyncPgConnection) -> QueryResult<Self> {
        crate_owner_invitations::table
            .filter(crate_owner_invitations::token.eq(token))
            .first::<Self>(conn)
            .await
    }

    pub async fn accept(self, conn: &mut AsyncPgConnection) -> Result<(), AcceptError> {
        if self.is_expired() {
            let crate_name: String = crates::table
                .find(self.crate_id)
                .select(crates::name)
                .first(conn)
                .await?;

            return Err(AcceptError::Expired { crate_name });
        }

        conn.transaction(|conn| {
            async move {
                CrateOwner::from_invite(&self).insert(conn).await?;

                diesel::delete(&self).execute(conn).await?;

                Ok(())
            }
            .scope_boxed()
        })
        .await
    }

    pub async fn decline(self, conn: &mut AsyncPgConnection) -> QueryResult<()> {
        // The check to prevent declining expired invitations is *explicitly* missing. We do not
        // care if an expired invitation is declined, as that just removes the invitation from the
        // database.

        diesel::delete(&self).execute(conn).await?;
        Ok(())
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at <= Utc::now()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AcceptError {
    #[error(transparent)]
    Diesel(#[from] diesel::result::Error),
    #[error("The invitation has expired")]
    Expired { crate_name: String },
}
