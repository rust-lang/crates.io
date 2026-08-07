use crate::app::AppState;
use crate::models::{CrateOwner, OwnerKind, PublicUser};
use crate::schema::{crate_downloads, crate_owners, crates};
use crate::util::errors::AppResult;
use crate::views::EncodablePublicUser;
use axum::Json;
use axum::extract::Path;
use bigdecimal::{BigDecimal, ToPrimitive};
use crates_io_database::fns::canon_username;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Serialize;

/// Response returned when getting a user by login.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UserGetResponse {
    pub user: EncodablePublicUser,
}

/// Find user by login.
#[utoipa::path(
    get,
    path = "/api/v1/users/{user}",
    params(
        ("user" = String, Path, description = "crates.io username"),
    ),
    tag = "users",
    responses(
        (status = 200, description = "Successful Response", body = inline(UserGetResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn find_user(
    state: AppState,
    Path(user_name): Path<String>,
) -> AppResult<Json<UserGetResponse>> {
    let mut conn = state.db_read_prefer_primary().await?;

    use crate::schema::users::dsl::{id, username};

    let name = canon_username(&user_name);
    let user: PublicUser = PublicUser::query()
        .filter(canon_username(username).eq(name))
        .order(id.desc())
        .first(&mut conn)
        .await?;

    Ok(Json(UserGetResponse { user: user.into() }))
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
    responses(
        (status = 200, description = "Successful Response", body = inline(StatsResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn get_user_stats(
    state: AppState,
    Path(user_id): Path<i32>,
) -> AppResult<Json<StatsResponse>> {
    let mut conn = state.db_read_prefer_primary().await?;

    use diesel::dsl::sum;
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
