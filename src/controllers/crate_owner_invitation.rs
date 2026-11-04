use crate::auth::AuthCheck;
use crate::auth::Authentication;
use crate::controllers::helpers::authorization::Rights;
use crate::controllers::helpers::pagination::{Page, PaginationOptions, PaginationQueryParams};
use crate::models::crate_owner_invitation::AcceptError;
use crate::models::token::{CrateScope, EndpointScope};
use crate::models::{Crate, CrateOwnerInvitation, PublicUser};
use crate::schema::{crate_owner_invitations, crates, users};
use crate::server::ServerContext;
use crate::util::RequestUtils;
use crate::util::errors::{AppResult, BoxedAppError, bad_request, custom, forbidden, internal};
use crate::util::no_store;
use crate::views::{
    CrateOwnerInvitationState, EncodableCrateOwnerInvitation, EncodablePublicUser,
    InvitationResponse,
};
use axum::Json;
use axum::extract::{FromRequestParts, Path, Query};
use axum_extra::TypedHeader;
use axum_extra::headers::CacheControl;
use chrono::Utc;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::Bool;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use http::StatusCode;
use http::request::Parts;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Query parameters for listing crate owner invitations.
#[derive(Debug, Deserialize, FromRequestParts, utoipa::IntoParams)]
#[from_request(via(Query))]
#[into_params(parameter_in = Query)]
pub struct CrateOwnerInvitationListQueryParams {
    /// Filter crate owner invitations by crate name.
    ///
    /// Only crate owners can query pending invitations for their crate.
    crate_name: Option<String>,

    /// The ID of the user who was invited to be a crate owner.
    ///
    /// This parameter needs to match the authenticated user's ID.
    invitee_id: Option<i32>,
}

/// List crate owner invitations for a crate or user.
///
/// Only pending invitations that haven't expired are returned.
///
/// Listing the invitations of a user accepts cookie authentication or API
/// tokens with the `crate-owner-invitations` scope. Legacy tokens without
/// scopes are refused. Tokens with crate scopes only see invitations for
/// matching crates.
/// Listing the invitations of a crate requires cookie authentication.
#[utoipa::path(
    get,
    path = "/api/v1/crate_owner_invitations",
    params(CrateOwnerInvitationListQueryParams, PaginationQueryParams),
    security(("cookie" = []), ("api_token" = [])),
    tag = "owners",
    responses(
        (status = 200, description = "Successful Response", body = inline(ListResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn list_crate_owner_invitations(
    ctx: ServerContext,
    params: CrateOwnerInvitationListQueryParams,
    req: Parts,
) -> AppResult<(TypedHeader<CacheControl>, Json<ListResponse>)> {
    let mut conn = ctx.db_read().await?;

    let filter = params.try_into()?;
    let auth = match filter {
        ListFilter::CrateName(_) => AuthCheck::only_cookie(),
        ListFilter::InviteeId(_) => AuthCheck::default()
            .with_endpoint_scope(EndpointScope::CrateOwnerInvitations)
            .allow_any_crate_scope()
            .reject_legacy_tokens(),
    };
    let auth = auth.check(&req, &mut conn).await?;
    let list = prepare_list(&ctx, &req, auth, filter, &conn).await?;
    Ok((no_store(), Json(list)))
}

/// List all crate owner invitations for a crate or user.
#[utoipa::path(
    get,
    path = "/api/private/crate_owner_invitations",
    params(CrateOwnerInvitationListQueryParams, PaginationQueryParams),
    security(("cookie" = [])),
    tag = "owners",
    extensions(("x-internal" = json!(true))),
    responses(
        (status = 200, description = "Successful Response", body = inline(PrivateListResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn private_list_crate_owner_invitations(
    ctx: ServerContext,
    params: CrateOwnerInvitationListQueryParams,
    req: Parts,
) -> AppResult<(TypedHeader<CacheControl>, Json<PrivateListResponse>)> {
    let mut conn = ctx.db_read().await?;
    let auth = AuthCheck::only_cookie().check(&req, &mut conn).await?;

    let filter = params.try_into()?;
    let list = prepare_list(&ctx, &req, auth, filter, &conn).await?;
    Ok((no_store(), Json(list.into())))
}

enum ListFilter {
    CrateName(String),
    InviteeId(i32),
}

impl TryFrom<CrateOwnerInvitationListQueryParams> for ListFilter {
    type Error = BoxedAppError;

    fn try_from(params: CrateOwnerInvitationListQueryParams) -> Result<Self, Self::Error> {
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
    ctx: &ServerContext,
    req: &Parts,
    auth: Authentication,
    filter: ListFilter,
    mut conn: &AsyncPgConnection,
) -> AppResult<ListResponse> {
    let pagination: PaginationOptions = PaginationOptions::builder()
        .enable_pages(false)
        .enable_seek(true)
        .gather(req)?;

    let user = auth.user();

    let mut crate_names = HashMap::new();

    let sql_filter: Box<dyn BoxableExpression<crate_owner_invitations::table, Pg, SqlType = Bool>> =
        match filter {
            ListFilter::CrateName(crate_name) => {
                // Only allow crate owners to query pending invitations for their crate.
                let krate: Crate = Crate::by_name(&crate_name).first(&mut conn).await?;
                let owners = krate.owners(conn).await?;
                let encryption = &ctx.config.token_encryption;
                if Rights::get(user, &*ctx.github, &owners, encryption).await? != Rights::Full {
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
                let filter = crate_owner_invitations::invited_user_id.eq(invitee_id);

                let crate_scopes = auth.api_token().and_then(|t| t.crate_scopes.as_deref());
                match crate_scopes.filter(|scopes| !scopes.is_empty()) {
                    Some(scopes) => {
                        let crate_ids = matching_crate_ids(invitee_id, scopes, conn).await?;
                        Box::new(filter.and(crate_owner_invitations::crate_id.eq_any(crate_ids)))
                    }
                    None => Box::new(filter),
                }
            }
        };

    // Load all the non-expired invitations matching the filter.
    let query = CrateOwnerInvitation::query()
        .filter(sql_filter)
        .filter(crate_owner_invitations::expires_at.gt(Utc::now()))
        .order_by((
            crate_owner_invitations::crate_id,
            crate_owner_invitations::invited_user_id,
        ))
        // We fetch one element over the page limit to then detect whether there is a next page.
        .limit(pagination.per_page + 1);

    // Load and paginate the results.
    let mut raw_invitations: Vec<CrateOwnerInvitation> = match pagination.page {
        Page::Unspecified => query.load(&mut conn).await?,
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
                .load(&mut conn)
                .await?
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
            .load(&mut conn)
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
        .collect::<Vec<_>>();

    let mut users = IndexMap::new();
    if !missing_users.is_empty() {
        let new_users: Vec<PublicUser> = PublicUser::query()
            .filter(users::id.eq_any(missing_users))
            .load(&mut conn)
            .await?;
        for user in new_users.into_iter() {
            users.insert(user.id, user);
        }
    }

    // Turn `CrateOwnerInvitation`s into `EncodablePrivateCrateOwnerInvitation`.
    let mut invitations = Vec::new();
    let mut users_in_response = HashSet::new();
    for invitation in raw_invitations.into_iter() {
        let crate_name = crate_names
            .get(&invitation.crate_id)
            .ok_or_else(|| internal(format!("missing crate with id {}", invitation.crate_id)))?;

        let state = CrateOwnerInvitationState::Pending;
        invitations.push(encode_invitation(&invitation, crate_name.clone(), state));
        users_in_response.insert(invitation.invited_user_id);
        users_in_response.insert(invitation.invited_by_user_id);
    }

    // Provide a stable response for the users list, only including the referenced users with
    // stable sorting.
    users.retain(|k, _| users_in_response.contains(k));
    users.sort_keys();

    Ok(ListResponse {
        crate_owner_invitations: invitations,
        users: users.into_iter().map(|(_, user)| user.into()).collect(),
        meta: ResponseMeta { next_page },
    })
}

/// Returns the IDs of the crates that `invitee_id` has invitations for and
/// that match one of the crate `scopes`.
async fn matching_crate_ids(
    invitee_id: i32,
    scopes: &[CrateScope],
    mut conn: &AsyncPgConnection,
) -> QueryResult<Vec<i32>> {
    let crates: Vec<(i32, String)> = crate_owner_invitations::table
        .inner_join(crates::table)
        .filter(crate_owner_invitations::invited_user_id.eq(invitee_id))
        .select((crates::id, crates::name))
        .load(&mut conn)
        .await?;

    let matches = |name: &str| scopes.iter().any(|scope| scope.matches(name));
    let crates = crates.into_iter().filter(|(_, name)| matches(name));
    Ok(crates.map(|(id, _)| id).collect())
}

fn encode_invitation(
    invitation: &CrateOwnerInvitation,
    crate_name: String,
    state: CrateOwnerInvitationState,
) -> EncodableCrateOwnerInvitation {
    EncodableCrateOwnerInvitation {
        id: invitation.id,
        state,
        invitee_id: invitation.invited_user_id,
        inviter_id: invitation.invited_by_user_id,
        crate_id: invitation.crate_id,
        crate_name,
        created_at: invitation.created_at,
        expires_at: invitation.expires_at,
    }
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ListResponse {
    /// The list of crate owner invitations.
    crate_owner_invitations: Vec<EncodableCrateOwnerInvitation>,

    /// The list of users referenced in the crate owner invitations.
    users: Vec<EncodablePublicUser>,

    #[schema(inline)]
    meta: ResponseMeta,
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

impl From<ListResponse> for PrivateListResponse {
    fn from(list: ListResponse) -> Self {
        Self {
            invitations: list.crate_owner_invitations,
            users: list.users,
            meta: list.meta,
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
struct ResponseMeta {
    /// Query parameter string to fetch the next page of results.
    #[schema(required = true, example = "?seek=c0ffee")]
    next_page: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct OwnerInvitation {
    #[schema(inline)]
    crate_owner_invite: InvitationResponse,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HandleResponse {
    #[schema(inline)]
    crate_owner_invitation: InvitationResponse,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateRequest {
    #[schema(inline)]
    crate_owner_invitation: InvitationUpdate,
}

#[derive(Deserialize, utoipa::ToSchema)]
struct InvitationUpdate {
    /// The new state of the invitation.
    state: CrateOwnerInvitationState,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct UpdateResponse {
    /// The invitation in its final state.
    crate_owner_invitation: EncodableCrateOwnerInvitation,
}

/// Accept or decline a crate owner invitation.
///
/// Only the invitee can accept or decline an invitation.
#[utoipa::path(
    patch,
    path = "/api/v1/crate_owner_invitations/{id}",
    params(
        ("id" = i64, Path, description = "ID of the crate owner invitation"),
    ),
    request_body(
        content = inline(UpdateRequest),
        examples(
            ("accept" = (value = json!({ "crate_owner_invitation": { "state": "accepted" } }))),
            ("decline" = (value = json!({ "crate_owner_invitation": { "state": "declined" } }))),
        ),
    ),
    security(("cookie" = [])),
    tag = "owners",
    responses(
        (status = 200, description = "Successful Response", body = inline(UpdateResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn update_crate_owner_invitation(
    ctx: ServerContext,
    Path(id): Path<i64>,
    parts: Parts,
    Json(request): Json<UpdateRequest>,
) -> AppResult<Json<UpdateResponse>> {
    let state = request.crate_owner_invitation.state;

    let mut conn = ctx.db_write().await?;
    let auth = AuthCheck::only_cookie().check(&parts, &mut conn).await?;

    let (invitation, crate_name) = crate_owner_invitations::table
        .find(id)
        .filter(crate_owner_invitations::invited_user_id.eq(auth.user_id()))
        .inner_join(crates::table)
        .select((CrateOwnerInvitation::as_select(), crates::name))
        .first(&mut conn)
        .await?;

    let crate_owner_invitation = encode_invitation(&invitation, crate_name, state);

    match state {
        CrateOwnerInvitationState::Accepted => invitation.accept(&mut conn).await?,
        CrateOwnerInvitationState::Declined => invitation.decline(&conn).await?,
        _ => return Err(bad_request("`state` must be `accepted` or `declined`")),
    }

    Ok(Json(UpdateResponse {
        crate_owner_invitation,
    }))
}

/// Accept or decline a crate owner invitation.
#[utoipa::path(
    put,
    path = "/api/v1/me/crate_owner_invitations/{crate_id}",
    params(
        ("crate_id" = i32, Path, description = "ID of the crate"),
    ),
    request_body = inline(OwnerInvitation),
    security(
        ("api_token" = []),
        ("cookie" = []),
    ),
    tag = "owners",
    responses(
        (status = 200, description = "Successful Response", body = inline(HandleResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn handle_crate_owner_invitation(
    ctx: ServerContext,
    parts: Parts,
    Json(crate_invite): Json<OwnerInvitation>,
) -> AppResult<Json<HandleResponse>> {
    let crate_invite = crate_invite.crate_owner_invite;

    let mut conn = ctx.db_write().await?;
    let user_id = AuthCheck::default()
        .check(&parts, &mut conn)
        .await?
        .user_id();
    let invitation =
        CrateOwnerInvitation::find_by_invitee_and_crate(user_id, crate_invite.crate_id, &conn)
            .await?;

    if crate_invite.accepted {
        invitation.accept(&mut conn).await?;
    } else {
        invitation.decline(&conn).await?;
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
    responses(
        (status = 200, description = "Successful Response", body = inline(HandleResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn accept_crate_owner_invitation_with_token(
    ctx: ServerContext,
    Path(token): Path<String>,
) -> AppResult<Json<HandleResponse>> {
    let mut conn = ctx.db_write().await?;
    let invitation = CrateOwnerInvitation::find_by_token(&token, &conn).await?;

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
            AcceptError::EmailNotVerified { crate_name } => {
                let detail = format!(
                    "You need to verify your email address before you can accept the invitation \
                    to become an owner of the {crate_name} crate.",
                );

                custom(StatusCode::FORBIDDEN, detail)
            }
        }
    }
}
