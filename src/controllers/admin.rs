use axum_template::RenderHtml;

use crate::{extractors::admin::AdminUser, views::admin::krates::CrateVersion};

use super::prelude::*;

/// Handles the `GET /admin/` route.
pub async fn index(app: AppState, _user: AdminUser) -> AppResult<impl IntoResponse> {
    conduit_compat(move || {
        use crate::schema::{crates, versions};

        let conn = &mut *app.db_read()?;

        // TODO: move to a new controller and redirect when hitting /admin/.
        // TODO: refactor into something that's not a spaghetti query.
        // TODO: pagination.
        // TODO: search.

        // XXX: can we send an iterator to RenderHtml?
        let recent_versions: Vec<CrateVersion> = versions::table
            .inner_join(crates::table)
            .select((
                versions::id,
                versions::num,
                versions::created_at,
                crates::name,
            ))
            .order(versions::created_at.desc())
            .limit(50)
            .load(conn)?
            .into_iter()
            .map(|(id, num, created_at, name)| -> CrateVersion {
                CrateVersion {
                    id,
                    num,
                    created_at,
                    name,
                }
            })
            .collect();

        Ok(RenderHtml(
            "crates",
            app.admin_engine.clone(),
            recent_versions,
        ))
    })
    .await
}
