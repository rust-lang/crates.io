use axum::extract::Path;
use axum_extra::json;
use axum_extra::response::ErasedJson;
use bigdecimal::{BigDecimal, ToPrimitive};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::app::AppState;
use crate::models::{CrateOwner, OwnerKind, User};
use crate::schema::{crate_downloads, crate_owners, crates};
use crate::sql::lower;
use crate::util::errors::AppResult;
use crate::views::EncodablePublicUser;

/// Find user by login.
#[utoipa::path(
    get,
    path = "/api/v1/users/{user}",
    tag = "users",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn find_user(state: AppState, Path(user_name): Path<String>) -> AppResult<ErasedJson> {
    let mut conn = state.db_read_prefer_primary().await?;

    use crate::schema::users::dsl::{gh_login, id, users};

    let name = lower(&user_name);
    let user: User = users
        .filter(lower(gh_login).eq(name))
        .order(id.desc())
        .first(&mut conn)
        .await?;

    Ok(json!({ "user": EncodablePublicUser::from(user) }))
}

/// Get user stats.
///
/// This currently only returns the total number of downloads for crates owned
/// by the user.
#[utoipa::path(
    get,
    path = "/api/v1/users/{id}/stats",
    tag = "users",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn get_user_stats(state: AppState, Path(user_id): Path<i32>) -> AppResult<ErasedJson> {
    let mut conn = state.db_read_prefer_primary().await?;

    use diesel::dsl::sum;
    use diesel_async::RunQueryDsl;

    let data = CrateOwner::by_owner_kind(OwnerKind::User)
        .inner_join(crates::table)
        .inner_join(crate_downloads::table.on(crates::id.eq(crate_downloads::crate_id)))
        .filter(crate_owners::owner_id.eq(user_id))
        .select(sum(crate_downloads::downloads))
        .first::<Option<BigDecimal>>(&mut conn)
        .await?
        .map(|d| d.to_u64().unwrap_or(u64::MAX))
        .unwrap_or(0);

    Ok(json!({ "total_downloads": data }))
}
