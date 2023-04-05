use axum::response::Html;

use crate::{auth::AuthCheck, models::User, schema::users};

use super::prelude::*;

/// Handles the `GET /admin` route.
pub async fn index(app: AppState, req: Parts) -> AppResult<Html<String>> {
    tracing::warn!("in admin index");

    conduit_compat(move || {
        let conn = &mut *app.db_read_prefer_primary()?;
        let user_id = AuthCheck::only_cookie().check(&req, conn)?.user_id();

        let user = users::table
            .find(user_id)
            .select(users::all_columns)
            .first::<User>(conn)?
            .admin()?;

        dbg!(user);

        Ok(Html("a wolf at the door".into()))
    })
    .await
}
