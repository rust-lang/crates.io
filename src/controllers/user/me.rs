use crate::auth::AuthCheck;
use crate::util::diesel::prelude::*;
use axum::extract::Path;
use axum::response::Response;
use axum::Json;
use axum_extra::json;
use axum_extra::response::ErasedJson;
use http::request::Parts;
use std::collections::HashMap;

use crate::app::AppState;
use crate::controllers::helpers::pagination::{Paginated, PaginationOptions};
use crate::controllers::helpers::{ok_true, Paginate};
use crate::models::krate::CrateName;
use crate::models::{CrateOwner, Follow, OwnerKind, User, Version, VersionOwnerAction};
use crate::schema::{crate_owners, crates, emails, follows, users, versions};
use crate::util::errors::{bad_request, AppResult};
use crate::util::BytesRequest;
use crate::views::{EncodableMe, EncodablePrivateUser, EncodableVersion, OwnedCrate};

/// Handles the `GET /me` route.
pub async fn me(app: AppState, req: Parts) -> AppResult<Json<EncodableMe>> {
    use diesel_async::RunQueryDsl;

    let mut conn = app.db_read_prefer_primary().await?;
    let user_id = AuthCheck::only_cookie()
        .check(&req, &mut conn)
        .await?
        .user_id();
    let (user, verified, email, verification_sent): (User, Option<bool>, Option<String>, bool) =
        users::table
            .find(user_id)
            .left_join(emails::table)
            .select((
                User::as_select(),
                emails::verified.nullable(),
                emails::email.nullable(),
                emails::token_generated_at.nullable().is_not_null(),
            ))
            .first(&mut conn)
            .await?;

    let owned_crates = CrateOwner::by_owner_kind(OwnerKind::User)
        .inner_join(crates::table)
        .filter(crate_owners::owner_id.eq(user_id))
        .select((crates::id, crates::name, crate_owners::email_notifications))
        .order(crates::name.asc())
        .load(&mut conn)
        .await?
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

/// Handles the `GET /me/updates` route.
pub async fn updates(app: AppState, req: Parts) -> AppResult<ErasedJson> {
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

    Ok(json!({
        "versions": versions,
        "meta": { "more": more },
    }))
}

/// Handles the `PUT /confirm/:email_token` route
pub async fn confirm_user_email(state: AppState, Path(token): Path<String>) -> AppResult<Response> {
    use diesel::update;
    use diesel_async::RunQueryDsl;

    let mut conn = state.db_write().await?;

    let updated_rows = update(emails::table.filter(emails::token.eq(&token)))
        .set(emails::verified.eq(true))
        .execute(&mut conn)
        .await?;

    if updated_rows == 0 {
        return Err(bad_request("Email belonging to token not found."));
    }

    ok_true()
}

/// Handles `PUT /me/email_notifications` route
pub async fn update_email_notifications(app: AppState, req: BytesRequest) -> AppResult<Response> {
    use diesel::pg::upsert::excluded;
    use diesel_async::RunQueryDsl;

    let (parts, body) = req.0.into_parts();

    #[derive(Deserialize)]
    struct CrateEmailNotifications {
        id: i32,
        email_notifications: bool,
    }

    let updates: HashMap<i32, bool> = serde_json::from_slice::<Vec<CrateEmailNotifications>>(&body)
        .map_err(|_| bad_request("invalid json request"))?
        .iter()
        .map(|c| (c.id, c.email_notifications))
        .collect();

    let mut conn = app.db_write().await?;
    let user_id = AuthCheck::default()
        .check(&parts, &mut conn)
        .await?
        .user_id();

    // Build inserts from existing crates belonging to the current user
    let to_insert = CrateOwner::by_owner_kind(OwnerKind::User)
        .filter(crate_owners::owner_id.eq(user_id))
        .select((
            crate_owners::crate_id,
            crate_owners::owner_id,
            crate_owners::owner_kind,
            crate_owners::email_notifications,
        ))
        .load(&mut conn)
        .await?
        .into_iter()
        // Remove records whose `email_notifications` will not change from their current value
        .map(
            |(c_id, o_id, o_kind, e_notifications): (i32, i32, i32, bool)| {
                let current_e_notifications = *updates.get(&c_id).unwrap_or(&e_notifications);
                (
                    crate_owners::crate_id.eq(c_id),
                    crate_owners::owner_id.eq(o_id),
                    crate_owners::owner_kind.eq(o_kind),
                    crate_owners::email_notifications.eq(current_e_notifications),
                )
            },
        )
        .collect::<Vec<_>>();

    // Upsert crate owners; this should only actually execute updates
    diesel::insert_into(crate_owners::table)
        .values(&to_insert)
        .on_conflict((
            crate_owners::crate_id,
            crate_owners::owner_id,
            crate_owners::owner_kind,
        ))
        .do_update()
        .set(crate_owners::email_notifications.eq(excluded(crate_owners::email_notifications)))
        .execute(&mut conn)
        .await?;

    ok_true()
}
