use chrono::NaiveDateTime;
use conduit::{Request, Response};
use diesel::prelude::*;
use serde_json;

use db::RequestTransaction;
use schema::{crate_owner_invitations, crate_owners, crates, users};
use user::RequestUser;
use util::errors::{human, CargoResult};
use util::RequestUtils;
use owner::{CrateOwner, OwnerKind};

/// The model representing a row in the `crate_owner_invitations` database table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Identifiable, Queryable)]
#[primary_key(invited_user_id, crate_id)]
pub struct CrateOwnerInvitation {
    pub invited_user_id: i32,
    pub invited_by_user_id: i32,
    pub crate_id: i32,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable, Clone, Copy, Debug)]
#[table_name = "crate_owner_invitations"]
pub struct NewCrateOwnerInvitation {
    pub invited_user_id: i32,
    pub invited_by_user_id: i32,
    pub crate_id: i32,
}

impl CrateOwnerInvitation {
    pub fn invited_by_username(&self, conn: &PgConnection) -> String {
        users::table
            .find(self.invited_by_user_id)
            .select(users::gh_login)
            .first(&*conn)
            .unwrap_or_else(|_| String::from("(unknown username)"))
    }

    pub fn crate_name(&self, conn: &PgConnection) -> String {
        crates::table
            .find(self.crate_id)
            .select(crates::name)
            .first(&*conn)
            .unwrap_or_else(|_| String::from("(unknown crate name)"))
    }

    pub fn encodable(self, conn: &PgConnection) -> EncodableCrateOwnerInvitation {
        EncodableCrateOwnerInvitation {
            invited_by_username: self.invited_by_username(conn),
            crate_name: self.crate_name(conn),
            crate_id: self.crate_id,
            created_at: self.created_at,
        }
    }
}

/// The serialization format for the `CrateOwnerInvitation` model.
#[derive(Deserialize, Serialize, Debug)]
pub struct EncodableCrateOwnerInvitation {
    pub invited_by_username: String,
    pub crate_name: String,
    pub crate_id: i32,
    pub created_at: NaiveDateTime,
}

/// Handles the `GET /me/crate_owner_invitations` route.
pub fn list(req: &mut Request) -> CargoResult<Response> {
    let conn = &*req.db_conn()?;
    let user_id = req.user()?.id;

    let crate_owner_invitations = crate_owner_invitations::table
        .filter(crate_owner_invitations::invited_user_id.eq(user_id))
        .load::<CrateOwnerInvitation>(&*conn)?
        .into_iter()
        .map(|i| i.encodable(conn))
        .collect();

    #[derive(Serialize)]
    struct R {
        crate_owner_invitations: Vec<EncodableCrateOwnerInvitation>,
    }
    Ok(req.json(&R {
        crate_owner_invitations,
    }))
}

#[derive(Deserialize)]
struct OwnerInvitation {
    crate_owner_invite: InvitationResponse,
}

#[derive(Deserialize, Serialize, Debug, Copy, Clone)]
pub struct InvitationResponse {
    pub crate_id: i32,
    pub accepted: bool,
}

/// Handles the `PUT /me/crate_owner_invitations/:crate_id` route.
pub fn handle_invite(req: &mut Request) -> CargoResult<Response> {
    let conn = &*req.db_conn()?;


    let mut body = String::new();
    req.body().read_to_string(&mut body)?;

    let crate_invite: OwnerInvitation =
        serde_json::from_str(&body).map_err(|_| human("invalid json request"))?;

    let crate_invite = crate_invite.crate_owner_invite;

    if crate_invite.accepted {
        accept_invite(req, conn, crate_invite)
    } else {
        decline_invite(req, conn, crate_invite)
    }
}

fn accept_invite(
    req: &mut Request,
    conn: &PgConnection,
    crate_invite: InvitationResponse,
) -> CargoResult<Response> {
    use diesel::{delete, insert};
    use diesel::pg::upsert::{do_update, OnConflictExtension};

    let user_id = req.user()?.id;
    let pending_crate_owner = crate_owner_invitations::table
        .filter(crate_owner_invitations::crate_id.eq(crate_invite.crate_id))
        .filter(crate_owner_invitations::invited_user_id.eq(user_id))
        .first::<CrateOwnerInvitation>(&*conn)?;

    let owner = CrateOwner {
        crate_id: crate_invite.crate_id,
        owner_id: user_id,
        created_by: pending_crate_owner.invited_by_user_id,
        owner_kind: OwnerKind::User as i32,
    };

    conn.transaction(|| {
        insert(&owner.on_conflict(
            crate_owners::table.primary_key(),
            do_update().set(crate_owners::deleted.eq(false)),
        )).into(crate_owners::table)
            .execute(conn)?;
        delete(
            crate_owner_invitations::table
                .filter(crate_owner_invitations::crate_id.eq(crate_invite.crate_id))
                .filter(crate_owner_invitations::invited_user_id.eq(user_id)),
        ).execute(conn)?;

        #[derive(Serialize)]
        struct R {
            crate_owner_invitation: InvitationResponse,
        }
        Ok(req.json(&R {
            crate_owner_invitation: crate_invite,
        }))
    })
}

fn decline_invite(
    req: &mut Request,
    conn: &PgConnection,
    crate_invite: InvitationResponse,
) -> CargoResult<Response> {
    use diesel::delete;
    let user_id = req.user()?.id;

    delete(
        crate_owner_invitations::table
            .filter(crate_owner_invitations::crate_id.eq(crate_invite.crate_id))
            .filter(crate_owner_invitations::invited_user_id.eq(user_id)),
    ).execute(conn)?;

    #[derive(Serialize)]
    struct R {
        crate_owner_invitation: InvitationResponse,
    }

    Ok(req.json(&R {
        crate_owner_invitation: crate_invite,
    }))
}
