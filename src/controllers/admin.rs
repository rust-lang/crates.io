use axum_template::RenderHtml;

use crate::{extractors::admin::AdminUser, views::admin::krates::CrateVersion};

use super::prelude::*;

/// Handles the `GET /admin/` route.
pub async fn index(app: AppState, _user: AdminUser) -> AppResult<impl IntoResponse> {
    conduit_compat(move || {
        use crate::schema::{crates, users, versions};

        let conn = &mut *app.db_read()?;

        // TODO: move to a new controller and redirect when hitting /admin/.
        // TODO: refactor into something that's not a spaghetti query.
        // TODO: pagination.
        // TODO: search.

        // XXX: can we send an iterator to RenderHtml?
        let recent_versions: Vec<CrateVersion> = versions::table
            .inner_join(crates::table)
            .inner_join(users::table)
            .select((
                versions::id,
                versions::num,
                versions::created_at,
                crates::name,
                users::gh_login,
                users::gh_avatar,
            ))
            .order(versions::created_at.desc())
            .limit(50)
            .load(conn)?
            .into_iter()
            .map(
                |(id, num, created_at, name, published_by_username, published_by_avatar)| {
                    CrateVersion {
                        id,
                        num,
                        created_at,
                        name,
                        published_by_username,
                        published_by_avatar,
                    }
                },
            )
            .collect();

        Ok(RenderHtml(
            "crates",
            app.admin_engine.clone(),
            recent_versions,
        ))
    })
    .await
}
