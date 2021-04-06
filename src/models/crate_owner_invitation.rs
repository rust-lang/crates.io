use chrono::{NaiveDateTime, Utc};
use diesel::prelude::*;

use crate::config::Config;
use crate::models::{CrateOwner, OwnerKind};
use crate::schema::{crate_owner_invitations, crate_owners, crates};
use crate::util::errors::{AppResult, OwnershipInvitationExpired};

#[derive(Debug)]
pub enum NewCrateOwnerInvitationOutcome {
    AlreadyExists,
    InviteCreated { plaintext_token: String },
}

/// The model representing a row in the `crate_owner_invitations` database table.
#[derive(Clone, Debug, PartialEq, Eq, Identifiable, Queryable)]
#[primary_key(invited_user_id, crate_id)]
pub struct CrateOwnerInvitation {
    pub invited_user_id: i32,
    pub invited_by_user_id: i32,
    pub crate_id: i32,
    pub created_at: NaiveDateTime,
    pub token: String,
    pub token_created_at: Option<NaiveDateTime>,
}

impl CrateOwnerInvitation {
    pub fn create(
        invited_user_id: i32,
        invited_by_user_id: i32,
        crate_id: i32,
        conn: &PgConnection,
    ) -> AppResult<NewCrateOwnerInvitationOutcome> {
        #[derive(Insertable, Clone, Copy, Debug)]
        #[table_name = "crate_owner_invitations"]
        struct NewRecord {
            invited_user_id: i32,
            invited_by_user_id: i32,
            crate_id: i32,
        }

        let res: Option<CrateOwnerInvitation> = diesel::insert_into(crate_owner_invitations::table)
            .values(&NewRecord {
                invited_user_id,
                invited_by_user_id,
                crate_id,
            })
            .on_conflict_do_nothing()
            .get_result(conn)
            .optional()?;

        Ok(match res {
            Some(record) => NewCrateOwnerInvitationOutcome::InviteCreated {
                plaintext_token: record.token,
            },
            None => NewCrateOwnerInvitationOutcome::AlreadyExists,
        })
    }

    pub fn find_by_id(user_id: i32, crate_id: i32, conn: &PgConnection) -> AppResult<Self> {
        Ok(crate_owner_invitations::table
            .find((user_id, crate_id))
            .first::<Self>(&*conn)?)
    }

    pub fn find_by_token(token: &str, conn: &PgConnection) -> AppResult<Self> {
        Ok(crate_owner_invitations::table
            .filter(crate_owner_invitations::token.eq(token))
            .first::<Self>(&*conn)?)
    }

    pub fn accept(self, conn: &PgConnection, config: &Config) -> AppResult<()> {
        if self.is_expired(config) {
            let crate_name = crates::table
                .find(self.crate_id)
                .select(crates::name)
                .first(conn)?;
            return Err(Box::new(OwnershipInvitationExpired { crate_name }));
        }

        conn.transaction(|| {
            diesel::insert_into(crate_owners::table)
                .values(&CrateOwner {
                    crate_id: self.crate_id,
                    owner_id: self.invited_user_id,
                    created_by: self.invited_by_user_id,
                    owner_kind: OwnerKind::User as i32,
                    email_notifications: true,
                })
                .on_conflict(crate_owners::table.primary_key())
                .do_update()
                .set(crate_owners::deleted.eq(false))
                .execute(conn)?;

            diesel::delete(&self).execute(conn)?;

            Ok(())
        })
    }

    pub fn decline(self, conn: &PgConnection) -> AppResult<()> {
        // The check to prevent declining expired invitations is *explicitly* missing. We do not
        // care if an expired invitation is declined, as that just removes the invitation from the
        // database.

        diesel::delete(&self).execute(conn)?;
        Ok(())
    }

    pub fn is_expired(&self, config: &Config) -> bool {
        self.expires_at(config) <= Utc::now().naive_utc()
    }

    pub fn expires_at(&self, config: &Config) -> NaiveDateTime {
        let days = chrono::Duration::days(config.ownership_invitations_expiration_days as i64);
        self.created_at + days
    }
}
