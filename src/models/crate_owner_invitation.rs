use chrono::{NaiveDateTime, Utc};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection};
use http::StatusCode;
use secrecy::SecretString;

use crate::config;
use crate::models::{CrateOwner, OwnerKind};
use crate::schema::{crate_owner_invitations, crate_owners, crates};
use crate::util::diesel::prelude::*;
use crate::util::errors::{custom, AppResult};

#[derive(Debug)]
pub enum NewCrateOwnerInvitationOutcome {
    AlreadyExists,
    InviteCreated { plaintext_token: SecretString },
}

/// The model representing a row in the `crate_owner_invitations` database table.
#[derive(Clone, Debug, Identifiable, Queryable)]
#[diesel(primary_key(invited_user_id, crate_id))]
pub struct CrateOwnerInvitation {
    pub invited_user_id: i32,
    pub invited_by_user_id: i32,
    pub crate_id: i32,
    pub created_at: NaiveDateTime,
    #[diesel(deserialize_as = String)]
    pub token: SecretString,
    pub token_created_at: Option<NaiveDateTime>,
}

impl CrateOwnerInvitation {
    pub async fn create(
        invited_user_id: i32,
        invited_by_user_id: i32,
        crate_id: i32,
        conn: &mut AsyncPgConnection,
        config: &config::Server,
    ) -> QueryResult<NewCrateOwnerInvitationOutcome> {
        use diesel_async::RunQueryDsl;

        #[derive(Insertable, Clone, Copy, Debug)]
        #[diesel(table_name = crate_owner_invitations, check_for_backend(diesel::pg::Pg))]
        struct NewRecord {
            invited_user_id: i32,
            invited_by_user_id: i32,
            crate_id: i32,
        }

        // Before actually creating the invite, check if an expired invitation already exists
        // and delete it from the database. This allows obtaining a new invite if the old one
        // expired, instead of returning "already exists".
        conn.transaction(|conn| {
            async move {
                // This does a SELECT FOR UPDATE + DELETE instead of a DELETE with a WHERE clause to
                // use the model's `is_expired` method, centralizing our expiration checking logic.
                let existing: Option<CrateOwnerInvitation> = crate_owner_invitations::table
                    .find((invited_user_id, crate_id))
                    .for_update()
                    .first(conn)
                    .await
                    .optional()?;

                if let Some(existing) = existing {
                    if existing.is_expired(config) {
                        diesel::delete(&existing).execute(conn).await?;
                    }
                }
                QueryResult::Ok(())
            }
            .scope_boxed()
        })
        .await?;

        let res: Option<CrateOwnerInvitation> = diesel::insert_into(crate_owner_invitations::table)
            .values(&NewRecord {
                invited_user_id,
                invited_by_user_id,
                crate_id,
            })
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

    pub async fn find_by_id(
        user_id: i32,
        crate_id: i32,
        conn: &mut AsyncPgConnection,
    ) -> QueryResult<Self> {
        use diesel_async::RunQueryDsl;

        crate_owner_invitations::table
            .find((user_id, crate_id))
            .first::<Self>(conn)
            .await
    }

    pub async fn find_by_token(token: &str, conn: &mut AsyncPgConnection) -> QueryResult<Self> {
        use diesel_async::RunQueryDsl;

        crate_owner_invitations::table
            .filter(crate_owner_invitations::token.eq(token))
            .first::<Self>(conn)
            .await
    }

    pub async fn accept(
        self,
        conn: &mut AsyncPgConnection,
        config: &config::Server,
    ) -> AppResult<()> {
        use diesel_async::scoped_futures::ScopedFutureExt;
        use diesel_async::{AsyncConnection, RunQueryDsl};

        if self.is_expired(config) {
            let crate_name: String = crates::table
                .find(self.crate_id)
                .select(crates::name)
                .first(conn)
                .await?;

            let detail = format!(
                "The invitation to become an owner of the {crate_name} crate expired. \
                Please reach out to an owner of the crate to request a new invitation.",
            );

            return Err(custom(StatusCode::GONE, detail));
        }

        conn.transaction(|conn| {
            async move {
                diesel::insert_into(crate_owners::table)
                    .values(&CrateOwner {
                        crate_id: self.crate_id,
                        owner_id: self.invited_user_id,
                        created_by: self.invited_by_user_id,
                        owner_kind: OwnerKind::User,
                        email_notifications: true,
                    })
                    .on_conflict(crate_owners::table.primary_key())
                    .do_update()
                    .set(crate_owners::deleted.eq(false))
                    .execute(conn)
                    .await?;

                diesel::delete(&self).execute(conn).await?;

                Ok(())
            }
            .scope_boxed()
        })
        .await
    }

    pub async fn decline(self, conn: &mut AsyncPgConnection) -> QueryResult<()> {
        use diesel_async::RunQueryDsl;

        // The check to prevent declining expired invitations is *explicitly* missing. We do not
        // care if an expired invitation is declined, as that just removes the invitation from the
        // database.

        diesel::delete(&self).execute(conn).await?;
        Ok(())
    }

    pub fn is_expired(&self, config: &config::Server) -> bool {
        self.expires_at(config) <= Utc::now().naive_utc()
    }

    pub fn expires_at(&self, config: &config::Server) -> NaiveDateTime {
        let days = chrono::Duration::days(config.ownership_invitations_expiration_days as i64);
        self.created_at + days
    }
}
