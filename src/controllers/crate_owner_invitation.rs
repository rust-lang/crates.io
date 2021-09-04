use super::frontend_prelude::*;

use crate::controllers::helpers::pagination::{Page, PaginationOptions};
use crate::controllers::util::AuthenticatedUser;
use crate::models::{Crate, CrateOwnerInvitation, Rights, User};
use crate::schema::{crate_owner_invitations, crates, users};
use crate::util::errors::{forbidden, internal};
use crate::views::{
    EncodableCrateOwnerInvitation, EncodableCrateOwnerInvitationV1, EncodablePublicUser,
    InvitationResponse,
};
use chrono::{Duration, Utc};
use diesel::{pg::Pg, sql_types::Bool};
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};

/// Handles the `GET /api/v1/me/crate_owner_invitations` route.
pub fn list(req: &mut dyn RequestExt) -> EndpointResult {
    let auth = req.authenticate()?.forbid_api_token_auth()?;
    let user_id = auth.user_id();

    let PrivateListResponse {
        invitations, users, ..
    } = prepare_list(req, auth, ListFilter::InviteeId(user_id))?;

    // The schema for the private endpoints is converted to the schema used by v1 endpoints.
    let crate_owner_invitations = invitations
        .into_iter()
        .map(|private| {
            Ok(EncodableCrateOwnerInvitationV1 {
                invited_by_username: users
                    .iter()
                    .find(|u| u.id == private.inviter_id)
                    .ok_or_else(|| internal(&format!("missing user {}", private.inviter_id)))?
                    .login
                    .clone(),
                invitee_id: private.invitee_id,
                inviter_id: private.inviter_id,
                crate_name: private.crate_name,
                crate_id: private.crate_id,
                created_at: private.created_at,
                expires_at: private.expires_at,
            })
        })
        .collect::<AppResult<Vec<EncodableCrateOwnerInvitationV1>>>()?;

    Ok(req.json(&json!({
        "crate_owner_invitations": crate_owner_invitations,
        "users": users,
    })))
}

/// Handles the `GET /api/private/crate_owner_invitations` route.
pub fn private_list(req: &mut dyn RequestExt) -> EndpointResult {
    let auth = req.authenticate()?.forbid_api_token_auth()?;

    let filter = if let Some(crate_name) = req.query().get("crate_name") {
        ListFilter::CrateName(crate_name.clone())
    } else if let Some(id) = req.query().get("invitee_id").and_then(|i| i.parse().ok()) {
        ListFilter::InviteeId(id)
    } else {
        return Err(bad_request("missing or invalid filter"));
    };

    let list = prepare_list(req, auth, filter)?;
    Ok(req.json(&list))
}

enum ListFilter {
    CrateName(String),
    InviteeId(i32),
}

fn prepare_list(
    req: &mut dyn RequestExt,
    auth: AuthenticatedUser,
    filter: ListFilter,
) -> AppResult<PrivateListResponse> {
    let pagination: PaginationOptions = PaginationOptions::builder()
        .enable_pages(false)
        .enable_seek(true)
        .gather(req)?;

    let user = auth.user();
    let conn = req.db_read_only()?;
    let config = &req.app().config;

    let mut crate_names = HashMap::new();
    let mut users = IndexMap::new();
    users.insert(user.id, user.clone());

    let sql_filter: Box<dyn BoxableExpression<crate_owner_invitations::table, Pg, SqlType = Bool>> =
        match filter {
            ListFilter::CrateName(crate_name) => {
                // Only allow crate owners to query pending invitations for their crate.
                let krate: Crate = Crate::by_name(&crate_name).first(&*conn)?;
                let owners = krate.owners(&*conn)?;
                if user.rights(req.app(), &owners)? != Rights::Full {
                    return Err(forbidden());
                }

                // Cache the crate name to avoid querying it from the database again
                crate_names.insert(krate.id, krate.name.clone());

                Box::new(crate_owner_invitations::crate_id.eq(krate.id))
            }
            ListFilter::InviteeId(invitee_id) => {
                if invitee_id != user.id {
                    return Err(forbidden());
                }
                Box::new(crate_owner_invitations::invited_user_id.eq(invitee_id))
            }
        };

    // Load all the non-expired invitations matching the filter.
    let expire_cutoff = Duration::days(config.ownership_invitations_expiration_days as i64);
    let query = crate_owner_invitations::table
        .filter(sql_filter)
        .filter(crate_owner_invitations::created_at.gt((Utc::now() - expire_cutoff).naive_utc()))
        .order_by((
            crate_owner_invitations::crate_id,
            crate_owner_invitations::invited_user_id,
        ))
        // We fetch one element over the page limit to then detect whether there is a next page.
        .limit(pagination.per_page as i64 + 1);

    // Load and paginate the results.
    let mut raw_invitations: Vec<CrateOwnerInvitation> = match pagination.page {
        Page::Unspecified => query.load(&*conn)?,
        Page::Seek(s) => {
            let seek_key: (i32, i32) = s.decode()?;
            query
                .filter(
                    crate_owner_invitations::crate_id.gt(seek_key.0).or(
                        crate_owner_invitations::crate_id
                            .eq(seek_key.0)
                            .and(crate_owner_invitations::invited_user_id.gt(seek_key.1)),
                    ),
                )
                .load(&*conn)?
        }
        Page::Numeric(_) => unreachable!("page-based pagination is disabled"),
    };
    let next_page = if raw_invitations.len() > pagination.per_page as usize {
        // We fetch `per_page + 1` to check if there are records for the next page. Since the last
        // element is not what the user wanted it's discarded.
        raw_invitations.pop();

        if let Some(last) = raw_invitations.last() {
            let mut params = IndexMap::new();
            params.insert(
                "seek".into(),
                crate::controllers::helpers::pagination::encode_seek((
                    last.crate_id,
                    last.invited_user_id,
                ))?,
            );
            Some(req.query_with_params(params))
        } else {
            None
        }
    } else {
        None
    };

    // Load all the related crates.
    let missing_crate_names = raw_invitations
        .iter()
        .map(|i| i.crate_id)
        .filter(|id| !crate_names.contains_key(id))
        .collect::<Vec<_>>();
    if !missing_crate_names.is_empty() {
        let new_names: Vec<(i32, String)> = crates::table
            .select((crates::id, crates::name))
            .filter(crates::id.eq_any(missing_crate_names))
            .load(&*conn)?;
        for (id, name) in new_names.into_iter() {
            crate_names.insert(id, name);
        }
    }

    // Load all the related users.
    let missing_users = raw_invitations
        .iter()
        .flat_map(|invite| {
            std::iter::once(invite.invited_user_id)
                .chain(std::iter::once(invite.invited_by_user_id))
        })
        .filter(|id| !users.contains_key(id))
        .collect::<Vec<_>>();
    if !missing_users.is_empty() {
        let new_users: Vec<User> = users::table
            .filter(users::id.eq_any(missing_users))
            .load(&*conn)?;
        for user in new_users.into_iter() {
            users.insert(user.id, user);
        }
    }

    // Turn `CrateOwnerInvitation`s into `EncodablePrivateCrateOwnerInvitation`.
    let config = &req.app().config;
    let mut invitations = Vec::new();
    let mut users_in_response = HashSet::new();
    for invitation in raw_invitations.into_iter() {
        invitations.push(EncodableCrateOwnerInvitation {
            invitee_id: invitation.invited_user_id,
            inviter_id: invitation.invited_by_user_id,
            crate_id: invitation.crate_id,
            crate_name: crate_names
                .get(&invitation.crate_id)
                .ok_or_else(|| internal(&format!("missing crate with id {}", invitation.crate_id)))?
                .clone(),
            created_at: invitation.created_at,
            expires_at: invitation.expires_at(config),
        });
        users_in_response.insert(invitation.invited_user_id);
        users_in_response.insert(invitation.invited_by_user_id);
    }

    // Provide a stable response for the users list, only including the referenced users with
    // stable sorting.
    users.retain(|k, _| users_in_response.contains(k));
    users.sort_keys();

    Ok(PrivateListResponse {
        invitations,
        users: users.into_iter().map(|(_, user)| user.into()).collect(),
        meta: ResponseMeta { next_page },
    })
}

#[derive(Serialize)]
struct PrivateListResponse {
    invitations: Vec<EncodableCrateOwnerInvitation>,
    users: Vec<EncodablePublicUser>,
    meta: ResponseMeta,
}

#[derive(Serialize)]
struct ResponseMeta {
    next_page: Option<String>,
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

    Ok(req.json(&json!({ "crate_owner_invitation": crate_invite })))
}

/// Handles the `PUT /api/v1/me/crate_owner_invitations/accept/:token` route.
pub fn handle_invite_with_token(req: &mut dyn RequestExt) -> EndpointResult {
    let config = &req.app().config;
    let conn = req.db_conn()?;
    let req_token = &req.params()["token"];

    let invitation = CrateOwnerInvitation::find_by_token(req_token, &conn)?;
    let crate_id = invitation.crate_id;
    invitation.accept(&conn, config)?;

    Ok(req.json(&json!({
        "crate_owner_invitation": {
            "crate_id": crate_id,
            "accepted": true,
        },
    })))
}
