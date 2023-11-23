use crate::controllers::frontend_prelude::*;

use crate::models::{CrateOwner, OwnerKind, User};
use crate::schema::{crate_owners, crates, users};
use crate::sql::lower;
use crate::views::EncodablePublicUser;

/// Handles the `GET /users/:user_id` route.
pub async fn show(state: AppState, Path(user_name): Path<String>) -> AppResult<Json<Value>> {
    spawn_blocking(move || {
        use self::users::dsl::{gh_login, id, users};

        let name = lower(&user_name);
        let conn = &mut *state.db_read_prefer_primary()?;
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
    spawn_blocking(move || {
        use diesel::dsl::sum;

        let conn = &mut *state.db_read_prefer_primary()?;

        let data: i64 = CrateOwner::by_owner_kind(OwnerKind::User)
            .inner_join(crates::table)
            .filter(crate_owners::owner_id.eq(user_id))
            .select(sum(crates::downloads))
            .first::<Option<i64>>(conn)?
            .unwrap_or(0);

        Ok(Json(json!({ "total_downloads": data })))
    })
    .await
}
