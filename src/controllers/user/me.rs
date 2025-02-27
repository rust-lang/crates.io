use crate::auth::AuthCheck;
use axum::Json;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use futures_util::FutureExt;
use http::request::Parts;

use crate::app::AppState;
use crate::controllers::helpers::Paginate;
use crate::controllers::helpers::pagination::{Paginated, PaginationOptions};
use crate::models::krate::CrateName;
use crate::models::{CrateOwner, Follow, OwnerKind, User, Version, VersionOwnerAction};
use crate::schema::{crate_owners, crates, emails, follows, users, versions};
use crate::util::errors::AppResult;
use crate::views::{EncodableMe, EncodablePrivateUser, EncodableVersion, OwnedCrate};

/// Get the currently authenticated user.
#[utoipa::path(
    get,
    path = "/api/v1/me",
    security(("cookie" = [])),
    tag = "users",
    responses((status = 200, description = "Successful Response", body = inline(EncodableMe))),
)]
pub async fn get_authenticated_user(app: AppState, req: Parts) -> AppResult<Json<EncodableMe>> {
    let mut conn = app.db_read_prefer_primary().await?;
    let user_id = AuthCheck::only_cookie()
        .check(&req, &mut conn)
        .await?
        .user_id();

    let ((user, verified, email, verification_sent), owned_crates) = tokio::try_join!(
        users::table
            .find(user_id)
            .left_join(emails::table)
            .select((
                User::as_select(),
                emails::verified.nullable(),
                emails::email.nullable(),
                emails::token_generated_at.nullable().is_not_null(),
            ))
            .first::<(User, Option<bool>, Option<String>, bool)>(&mut conn)
            .boxed(),
        CrateOwner::by_owner_kind(OwnerKind::User)
            .inner_join(crates::table)
            .filter(crate_owners::owner_id.eq(user_id))
            .select((crates::id, crates::name, crate_owners::email_notifications))
            .order(crates::name.asc())
            .load(&mut conn)
            .boxed()
    )?;

    let owned_crates = owned_crates
        .into_iter()
        .map(|(id, name, email_notifications)| OwnedCrate {
            id,
            name,
            email_notifications,
        })
        .collect();

    let verified = verified.unwrap_or(false);
    let verification_sent = verified || verification_sent;
    Ok(Json(EncodableMe {
        user: EncodablePrivateUser::from(user, email, verified, verification_sent),
        owned_crates,
    }))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UpdatesResponse {
    /// The list of recent versions of crates that the authenticated user follows.
    pub versions: Vec<EncodableVersion>,

    #[schema(inline)]
    pub meta: UpdatesResponseMeta,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UpdatesResponseMeta {
    /// Whether there are more versions to be loaded.
    pub more: bool,
}

/// List versions of crates that the authenticated user follows.
#[utoipa::path(
    get,
    path = "/api/v1/me/updates",
    security(("cookie" = [])),
    tag = "versions",
    responses((status = 200, description = "Successful Response", body = inline(UpdatesResponse))),
)]
pub async fn get_authenticated_user_updates(
    app: AppState,
    req: Parts,
) -> AppResult<Json<UpdatesResponse>> {
    let mut conn = app.db_read_prefer_primary().await?;
    let auth = AuthCheck::only_cookie().check(&req, &mut conn).await?;

    let user = auth.user();

    let followed_crates = Follow::belonging_to(user).select(follows::crate_id);
    let query = versions::table
        .inner_join(crates::table)
        .left_outer_join(users::table)
        .filter(crates::id.eq_any(followed_crates))
        .order(versions::created_at.desc())
        .select(<(Version, CrateName, Option<User>)>::as_select())
        .pages_pagination(PaginationOptions::builder().gather(&req)?);

    let data: Paginated<(Version, CrateName, Option<User>)> = query.load(&mut conn).await?;

    let more = data.next_page_params().is_some();
    let versions = data.iter().map(|(v, ..)| v).collect::<Vec<_>>();
    let actions = VersionOwnerAction::for_versions(&mut conn, &versions).await?;
    let data = data
        .into_iter()
        .zip(actions)
        .map(|((v, cn, pb), voas)| (v, cn, pb, voas));

    let versions = data
        .into_iter()
        .map(|(version, crate_name, published_by, actions)| {
            EncodableVersion::from(version, &crate_name.name, published_by, actions)
        })
        .collect::<Vec<_>>();

    Ok(Json(UpdatesResponse {
        versions,
        meta: UpdatesResponseMeta { more },
    }))
}
