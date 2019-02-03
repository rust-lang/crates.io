use super::prelude::*;

use crate::models::{CrateOwner, CrateOwnerInvitation, OwnerKind};
use crate::schema::{crate_owner_invitations, crate_owners};
use crate::views::{EncodableCrateOwnerInvitation, InvitationResponse};

/// Handles the `GET /me/crate_owner_invitations` route.
pub fn list(req: &mut dyn Request) -> CargoResult<Response> {
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

/// Handles the `PUT /me/crate_owner_invitations/:crate_id` route.
pub fn handle_invite(req: &mut dyn Request) -> CargoResult<Response> {
    let mut body = String::new();
    req.body().read_to_string(&mut body)?;

    let conn = &*req.db_conn()?;

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
    req: &dyn Request,
    conn: &PgConnection,
    crate_invite: InvitationResponse,
) -> CargoResult<Response> {
    use diesel::{delete, insert_into};

    let user_id = req.user()?.id;

    conn.transaction(|| {
        let pending_crate_owner = crate_owner_invitations::table
            .find((user_id, crate_invite.crate_id))
            .first::<CrateOwnerInvitation>(&*conn)?;

        insert_into(crate_owners::table)
            .values(&CrateOwner {
                crate_id: crate_invite.crate_id,
                owner_id: user_id,
                created_by: pending_crate_owner.invited_by_user_id,
                owner_kind: OwnerKind::User as i32,
            })
            .on_conflict(crate_owners::table.primary_key())
            .do_update()
            .set(crate_owners::deleted.eq(false))
            .execute(conn)?;
        delete(crate_owner_invitations::table.find((user_id, crate_invite.crate_id)))
            .execute(conn)?;

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
    req: &dyn Request,
    conn: &PgConnection,
    crate_invite: InvitationResponse,
) -> CargoResult<Response> {
    use diesel::delete;

    let user_id = req.user()?.id;

    delete(crate_owner_invitations::table.find((user_id, crate_invite.crate_id))).execute(conn)?;

    #[derive(Serialize)]
    struct R {
        crate_owner_invitation: InvitationResponse,
    }

    Ok(req.json(&R {
        crate_owner_invitation: crate_invite,
    }))
}
