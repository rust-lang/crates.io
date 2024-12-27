use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::controllers::helpers::ok_true;
use crate::models::{CrateOwner, OwnerKind};
use crate::schema::crate_owners;
use crate::util::errors::AppResult;
use axum::response::Response;
use axum::Json;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::request::Parts;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct CrateEmailNotifications {
    id: i32,
    email_notifications: bool,
}

/// Update email notification settings for the authenticated user.
///
/// This endpoint was implemented for an experimental feature that was never
/// fully implemented. It is now deprecated and will be removed in the future.
#[utoipa::path(
    put,
    path = "/api/v1/me/email_notifications",
    security(
        ("api_token" = []),
        ("cookie" = []),
    ),
    tag = "users",
    responses((status = 200, description = "Successful Response")),
)]
#[deprecated]
pub async fn update_email_notifications(
    app: AppState,
    parts: Parts,
    Json(updates): Json<Vec<CrateEmailNotifications>>,
) -> AppResult<Response> {
    use diesel::pg::upsert::excluded;

    let updates: HashMap<i32, bool> = updates
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
