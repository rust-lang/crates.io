use super::frontend_prelude::*;

use crate::models::{CrateOwnerInvitation, User};
use crate::schema::{crate_owner_invitations, crates, users};
use crate::views::{EncodableCrateOwnerInvitation, EncodablePublicUser, InvitationResponse};
use diesel::dsl::any;
use std::collections::HashMap;

/// Handles the `GET /api/v1/me/crate_owner_invitations` route.
pub fn list(req: &mut dyn RequestExt) -> EndpointResult {
    // Ensure that the user is authenticated
    let user = req.authenticate()?.forbid_api_token_auth()?.user();

    // Load all pending invitations for the user
    let conn = &*req.db_read_only()?;
    let crate_owner_invitations: Vec<CrateOwnerInvitation> = crate_owner_invitations::table
        .filter(crate_owner_invitations::invited_user_id.eq(user.id))
        .load(&*conn)?;

    // Make a list of all related users
    let user_ids: Vec<_> = crate_owner_invitations
        .iter()
        .map(|invitation| invitation.invited_by_user_id)
        .collect();

    // Load all related users
    let users: Vec<User> = users::table
        .filter(users::id.eq(any(user_ids)))
        .load(conn)?;

    let users: HashMap<i32, User> = users.into_iter().map(|user| (user.id, user)).collect();

    // Make a list of all related crates
    let crate_ids: Vec<_> = crate_owner_invitations
        .iter()
        .map(|invitation| invitation.crate_id)
        .collect();

    // Load all related crates
    let crates: Vec<_> = crates::table
        .select((crates::id, crates::name))
        .filter(crates::id.eq(any(crate_ids)))
        .load(conn)?;

    let crates: HashMap<i32, String> = crates.into_iter().collect();

    // Turn `CrateOwnerInvitation` list into `EncodableCrateOwnerInvitation` list
    let config = &req.app().config;
    let crate_owner_invitations = crate_owner_invitations
        .into_iter()
        .filter(|i| !i.is_expired(config))
        .map(|invitation| {
            let inviter_id = invitation.invited_by_user_id;
            let inviter_name = users
                .get(&inviter_id)
                .map(|user| user.gh_login.clone())
                .unwrap_or_default();

            let crate_name = crates
                .get(&invitation.crate_id)
                .cloned()
                .unwrap_or_else(|| String::from("(unknown crate name)"));

            let expires_at = invitation.expires_at(config);
            EncodableCrateOwnerInvitation::from(invitation, inviter_name, crate_name, expires_at)
        })
        .collect();

    // Turn `User` list into `EncodablePublicUser` list
    let users = users
        .into_iter()
        .map(|(_, user)| EncodablePublicUser::from(user))
        .collect();

    #[derive(Serialize)]
    struct R {
        crate_owner_invitations: Vec<EncodableCrateOwnerInvitation>,
        users: Vec<EncodablePublicUser>,
    }
    Ok(req.json(&R {
        crate_owner_invitations,
        users,
    }))
}

#[derive(Deserialize)]
struct OwnerInvitation {
    crate_owner_invite: InvitationResponse,
}

/// Handles the `PUT /api/v1/me/crate_owner_invitations/:crate_id` route.
pub fn handle_invite(req: &mut dyn RequestExt) -> EndpointResult {
    let mut body = String::new();
    req.body().read_to_string(&mut body)?;

    let crate_invite: OwnerInvitation =
        serde_json::from_str(&body).map_err(|_| bad_request("invalid json request"))?;

    let crate_invite = crate_invite.crate_owner_invite;
    let user_id = req.authenticate()?.user_id();
    let conn = &*req.db_conn()?;
    let config = &req.app().config;

    let invitation = CrateOwnerInvitation::find_by_id(user_id, crate_invite.crate_id, conn)?;
    if crate_invite.accepted {
        invitation.accept(conn, config)?;
    } else {
        invitation.decline(conn)?;
    }

    #[derive(Serialize)]
    struct R {
        crate_owner_invitation: InvitationResponse,
    }
    Ok(req.json(&R {
        crate_owner_invitation: crate_invite,
    }))
}

/// Handles the `PUT /api/v1/me/crate_owner_invitations/accept/:token` route.
pub fn handle_invite_with_token(req: &mut dyn RequestExt) -> EndpointResult {
    let config = &req.app().config;
    let conn = req.db_conn()?;
    let req_token = &req.params()["token"];

    let invitation = CrateOwnerInvitation::find_by_token(req_token, &conn)?;
    let crate_id = invitation.crate_id;
    invitation.accept(&conn, config)?;

    #[derive(Serialize)]
    struct R {
        crate_owner_invitation: InvitationResponse,
    }
    Ok(req.json(&R {
        crate_owner_invitation: InvitationResponse {
            crate_id,
            accepted: true,
        },
    }))
}
