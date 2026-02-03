use crate::app::AppState;
use crate::models::{CrateOwner, OwnerKind, User, UserWithLinkedAccounts};
use crate::schema::{crate_downloads, crate_owners, crates};
use crate::util::errors::{AppResult, not_found};
use crate::views::EncodablePublicUser;
use axum::Json;
use axum::extract::Path;
use bigdecimal::{BigDecimal, ToPrimitive};
use serde::Serialize;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct GetResponse {
    pub user: EncodablePublicUser,
}

/// Find user by login.
#[utoipa::path(
    get,
    path = "/api/v1/users/{user}",
    params(
        ("user" = String, Path, description = "Login name of the user"),
    ),
    tag = "users",
    responses((status = 200, description = "Successful Response", body = inline(GetResponse))),
)]
pub async fn find_user(
    state: AppState,
    Path(user_name): Path<String>,
) -> AppResult<Json<GetResponse>> {
    let mut conn = state.db_read_prefer_primary().await?;

    let users = User::find_all_by_login(&mut conn, &user_name).await?;
    let mut users = UserWithLinkedAccounts::find_all_by_users(&mut conn, users)
        .await?
        .into_iter();

    let user = users.next().ok_or_else(|| not_found())?;

    Ok(Json(GetResponse { user: user.into() }))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct StatsResponse {
    /// The total number of downloads for crates owned by the user.
    #[schema(example = 123_456_789)]
    pub total_downloads: u64,
}

/// Get user stats.
///
/// This currently only returns the total number of downloads for crates owned
/// by the user.
#[utoipa::path(
    get,
    path = "/api/v1/users/{id}/stats",
    params(
        ("id" = i32, Path, description = "ID of the user"),
    ),
    tag = "users",
    responses((status = 200, description = "Successful Response", body = inline(StatsResponse))),
)]
pub async fn get_user_stats(
    state: AppState,
    Path(user_id): Path<i32>,
) -> AppResult<Json<StatsResponse>> {
    let mut conn = state.db_read_prefer_primary().await?;

    use diesel::{dsl::sum, prelude::*};
    use diesel_async::RunQueryDsl;

    let total_downloads = CrateOwner::by_owner_kind(OwnerKind::User)
        .inner_join(crates::table)
        .inner_join(crate_downloads::table.on(crates::id.eq(crate_downloads::crate_id)))
        .filter(crate_owners::owner_id.eq(user_id))
        .select(sum(crate_downloads::downloads))
        .first::<Option<BigDecimal>>(&mut conn)
        .await?
        .map(|d| d.to_u64().unwrap_or(u64::MAX))
        .unwrap_or(0);

    Ok(Json(StatsResponse { total_downloads }))
}
