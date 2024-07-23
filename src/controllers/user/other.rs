use crate::controllers::frontend_prelude::*;
use bigdecimal::{BigDecimal, ToPrimitive};
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;

use crate::models::{CrateOwner, OwnerKind, User};
use crate::schema::{crate_downloads, crate_owners, crates, users};
use crate::sql::lower;
use crate::views::EncodablePublicUser;

/// Handles the `GET /users/:user_id` route.
pub async fn show(state: AppState, Path(user_name): Path<String>) -> AppResult<Json<Value>> {
    let conn = state.db_read_prefer_primary().await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        use self::users::dsl::{gh_login, id, users};

        let name = lower(&user_name);
        let user: User = users
            .filter(lower(gh_login).eq(name))
            .order(id.desc())
            .first(conn)?;

        Ok(Json(json!({ "user": EncodablePublicUser::from(user) })))
    })
    .await
}

/// Handles the `GET /users/:user_id/stats` route.
pub async fn stats(state: AppState, Path(user_id): Path<i32>) -> AppResult<Json<Value>> {
    let conn = state.db_read_prefer_primary().await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        use diesel::dsl::sum;

        let data = CrateOwner::by_owner_kind(OwnerKind::User)
            .inner_join(crates::table)
            .inner_join(crate_downloads::table.on(crates::id.eq(crate_downloads::crate_id)))
            .filter(crate_owners::owner_id.eq(user_id))
            .select(sum(crate_downloads::downloads))
            .first::<Option<BigDecimal>>(conn)?
            .map(|d| d.to_u64().unwrap_or(u64::MAX))
            .unwrap_or(0);

        Ok(Json(json!({ "total_downloads": data })))
    })
    .await
}
