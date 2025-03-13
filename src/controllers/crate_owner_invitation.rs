use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::auth::Authentication;
use crate::controllers::helpers::authorization::Rights;
use crate::controllers::helpers::pagination::{Page, PaginationOptions, PaginationQueryParams};
use crate::models::crate_owner_invitation::AcceptError;
use crate::models::{Crate, CrateOwnerInvitation, User};
use crate::schema::{crate_owner_invitations, crates, users};
use crate::util::RequestUtils;
use crate::util::errors::{AppResult, BoxedAppError, bad_request, custom, forbidden, internal};
use crate::views::{
    EncodableCrateOwnerInvitation, EncodableCrateOwnerInvitationV1, EncodablePublicUser,
    InvitationResponse,
};
use axum::Json;
use axum::extract::{FromRequestParts, Path, Query};
use chrono::Utc;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::Bool;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use http::StatusCode;
use http::request::Parts;
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};

#[derive(Serialize, utoipa::ToSchema)]
pub struct LegacyListResponse {
    /// The list of crate owner invitations.
    crate_owner_invitations: Vec<EncodableCrateOwnerInvitationV1>,

    /// The list of users referenced in the crate owner invitations.
    users: Vec<EncodablePublicUser>,
}

/// List all crate owner invitations for the authenticated user.
#[utoipa::path(
    get,
    path = "/api/v1/me/crate_owner_invitations",
    security(("cookie" = [])),
    tag = "owners",
    responses((status = 200, description = "Successful Response", body = inline(LegacyListResponse))),
)]
pub async fn list_crate_owner_invitations_for_user(
    app: AppState,
    req: Parts,
) -> AppResult<Json<LegacyListResponse>> {
    let mut conn = app.db_read().await?;
    let auth = AuthCheck::only_cookie().check(&req, &mut conn).await?;

    let user_id = auth.user_id();

    let PrivateListResponse {
        invitations, users, ..
    } = prepare_list(&app, &req, auth, ListFilter::InviteeId(user_id), &mut conn).await?;

    // The schema for the private endpoints is converted to the schema used by v1 endpoints.
    let crate_owner_invitations = invitations
        .into_iter()
        .map(|private| {
            Ok(EncodableCrateOwnerInvitationV1 {
                invited_by_username: users
                    .iter()
                    .find(|u| u.id == private.inviter_id)
                    .ok_or_else(|| internal(format!("missing user {}", private.inviter_id)))?
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

    Ok(Json(LegacyListResponse {
        crate_owner_invitations,
        users,
    }))
}

#[derive(Debug, Deserialize, FromRequestParts, utoipa::IntoParams)]
#[from_request(via(Query))]
#[into_params(parameter_in = Query)]
pub struct ListQueryParams {
    /// Filter crate owner invitations by crate name.
    ///
    /// Only crate owners can query pending invitations for their crate.
    crate_name: Option<String>,

    /// The ID of the user who was invited to be a crate owner.
    ///
    /// This parameter needs to match the authenticated user's ID.
    invitee_id: Option<i32>,
}

/// List all crate owner invitations for a crate or user.
#[utoipa::path(
    get,
    path = "/api/private/crate_owner_invitations",
    params(ListQueryParams, PaginationQueryParams),
    security(("cookie" = [])),
    tag = "owners",
    responses((status = 200, description = "Successful Response", body = inline(PrivateListResponse))),
)]
pub async fn list_crate_owner_invitations(
    app: AppState,
    params: ListQueryParams,
    req: Parts,
) -> AppResult<Json<PrivateListResponse>> {
    let mut conn = app.db_read().await?;
    let auth = AuthCheck::only_cookie().check(&req, &mut conn).await?;

    let filter = params.try_into()?;
    let list = prepare_list(&app, &req, auth, filter, &mut conn).await?;
    Ok(Json(list))
}

enum ListFilter {
    CrateName(String),
    InviteeId(i32),
}

impl TryFrom<ListQueryParams> for ListFilter {
    type Error = BoxedAppError;

    fn try_from(params: ListQueryParams) -> Result<Self, Self::Error> {
        let filter = if let Some(crate_name) = params.crate_name {
            ListFilter::CrateName(crate_name.clone())
        } else if let Some(id) = params.invitee_id {
            ListFilter::InviteeId(id)
        } else {
            return Err(bad_request("missing or invalid filter"));
        };

        Ok(filter)
    }
}

async fn prepare_list(
    state: &AppState,
    req: &Parts,
    auth: Authentication,
    filter: ListFilter,
    conn: &mut AsyncPgConnection,
) -> AppResult<PrivateListResponse> {
    let pagination: PaginationOptions = PaginationOptions::builder()
        .enable_pages(false)
        .enable_seek(true)
        .gather(req)?;

    let user = auth.user();

    let config = &state.config;

    let mut crate_names = HashMap::new();
    let mut users = IndexMap::new();
    users.insert(user.id, user.clone());

    let sql_filter: Box<dyn BoxableExpression<crate_owner_invitations::table, Pg, SqlType = Bool>> =
        match filter {
            ListFilter::CrateName(crate_name) => {
                // Only allow crate owners to query pending invitations for their crate.
                let krate: Crate = Crate::by_name(&crate_name).first(conn).await?;
                let owners = krate.owners(conn).await?;
                if Rights::get(user, &*state.github, &owners).await? != Rights::Full {
                    let detail = "only crate owners can query pending invitations for their crate";
                    return Err(forbidden(detail));
                }

                // Cache the crate name to avoid querying it from the database again
                crate_names.insert(krate.id, krate.name.clone());

                Box::new(crate_owner_invitations::crate_id.eq(krate.id))
            }
            ListFilter::InviteeId(invitee_id) => {
                if invitee_id != user.id {
                    let detail = "only the invitee can query their pending invitations";
                    return Err(forbidden(detail));
                }
                Box::new(crate_owner_invitations::invited_user_id.eq(invitee_id))
            }
        };

    // Load all the non-expired invitations matching the filter.
    let expire_cutoff = config.ownership_invitations_expiration;
    let query = crate_owner_invitations::table
        .filter(sql_filter)
        .filter(crate_owner_invitations::created_at.gt((Utc::now() - expire_cutoff).naive_utc()))
        .order_by((
            crate_owner_invitations::crate_id,
            crate_owner_invitations::invited_user_id,
        ))
        // We fetch one element over the page limit to then detect whether there is a next page.
        .limit(pagination.per_page + 1);

    // Load and paginate the results.
    let mut raw_invitations: Vec<CrateOwnerInvitation> = match pagination.page {
        Page::Unspecified => query.load(conn).await?,
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
                .load(conn)
                .await?
        }
        Page::SeekBackward(_) => unreachable!("seek-backward is disabled"),
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
            .load(conn)
            .await?;
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
            .load(conn)
            .await?;
        for user in new_users.into_iter() {
            users.insert(user.id, user);
        }
    }

    // Turn `CrateOwnerInvitation`s into `EncodablePrivateCrateOwnerInvitation`.
    let mut invitations = Vec::new();
    let mut users_in_response = HashSet::new();
    for invitation in raw_invitations.into_iter() {
        invitations.push(EncodableCrateOwnerInvitation {
            invitee_id: invitation.invited_user_id,
            inviter_id: invitation.invited_by_user_id,
            crate_id: invitation.crate_id,
            crate_name: crate_names
                .get(&invitation.crate_id)
                .ok_or_else(|| internal(format!("missing crate with id {}", invitation.crate_id)))?
                .clone(),
            created_at: invitation.created_at,
            expires_at: invitation.expires_at,
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

#[derive(Serialize, utoipa::ToSchema)]
pub struct PrivateListResponse {
    /// The list of crate owner invitations.
    invitations: Vec<EncodableCrateOwnerInvitation>,

    /// The list of users referenced in the crate owner invitations.
    users: Vec<EncodablePublicUser>,

    #[schema(inline)]
    meta: ResponseMeta,
}

#[derive(Serialize, utoipa::ToSchema)]
struct ResponseMeta {
    /// Query parameter string to fetch the next page of results.
    #[schema(example = "?seek=c0ffee")]
    next_page: Option<String>,
}

#[derive(Deserialize)]
pub struct OwnerInvitation {
    crate_owner_invite: InvitationResponse,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HandleResponse {
    #[schema(inline)]
    crate_owner_invitation: InvitationResponse,
}

/// Accept or decline a crate owner invitation.
#[utoipa::path(
    put,
    path = "/api/v1/me/crate_owner_invitations/{crate_id}",
    params(
        ("crate_id" = i32, Path, description = "ID of the crate"),
    ),
    security(
        ("api_token" = []),
        ("cookie" = []),
    ),
    tag = "owners",
    responses((status = 200, description = "Successful Response", body = inline(HandleResponse))),
)]
pub async fn handle_crate_owner_invitation(
    state: AppState,
    parts: Parts,
    Json(crate_invite): Json<OwnerInvitation>,
) -> AppResult<Json<HandleResponse>> {
    let crate_invite = crate_invite.crate_owner_invite;

    let mut conn = state.db_write().await?;
    let user_id = AuthCheck::default()
        .check(&parts, &mut conn)
        .await?
        .user_id();
    let invitation =
        CrateOwnerInvitation::find_by_id(user_id, crate_invite.crate_id, &mut conn).await?;

    if crate_invite.accepted {
        invitation.accept(&mut conn).await?;
    } else {
        invitation.decline(&mut conn).await?;
    }

    Ok(Json(HandleResponse {
        crate_owner_invitation: crate_invite,
    }))
}

/// Accept a crate owner invitation with a token.
#[utoipa::path(
    put,
    path = "/api/v1/me/crate_owner_invitations/accept/{token}",
    params(
        ("token" = String, Path, description = "Secret token sent to the user's email address"),
    ),
    tag = "owners",
    responses((status = 200, description = "Successful Response", body = inline(HandleResponse))),
)]
pub async fn accept_crate_owner_invitation_with_token(
    state: AppState,
    Path(token): Path<String>,
) -> AppResult<Json<HandleResponse>> {
    let mut conn = state.db_write().await?;
    let invitation = CrateOwnerInvitation::find_by_token(&token, &mut conn).await?;

    let crate_id = invitation.crate_id;
    invitation.accept(&mut conn).await?;

    let crate_owner_invitation = InvitationResponse {
        crate_id,
        accepted: true,
    };

    Ok(Json(HandleResponse {
        crate_owner_invitation,
    }))
}

impl From<AcceptError> for BoxedAppError {
    fn from(error: AcceptError) -> Self {
        match error {
            AcceptError::Diesel(error) => error.into(),
            AcceptError::Expired { crate_name } => {
                let detail = format!(
                    "The invitation to become an owner of the {crate_name} crate expired. \
                    Please reach out to an owner of the crate to request a new invitation.",
                );

                custom(StatusCode::GONE, detail)
            }
        }
    }
}
