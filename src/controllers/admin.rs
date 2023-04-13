use super::prelude::*;
use axum::extract::Query;

use crate::{
    extractors::admin::AdminUser,
    models::{Crate, User, Version},
    views::admin::crates::{render_versions, CrateVersion},
};

/// Handles the `GET /admin/` route.
pub async fn index(_user: AdminUser) -> impl IntoResponse {
    redirect("/admin/crates/".to_string())
}

#[derive(Deserialize)]
pub struct CrateQuery {
    q: Option<String>,
    page: Option<u32>,
}

/// Handles the `GET /admin/crates/` route.
pub async fn crates(
    app: AppState,
    q: Query<CrateQuery>,
    _user: AdminUser,
) -> AppResult<impl IntoResponse> {
    const PER_PAGE: i64 = 50;

    conduit_compat(move || {
        use crate::schema::{crates, users, versions};

        let conn = &mut *app.db_read()?;

        let mut query = versions::table
            .inner_join(crates::table)
            .inner_join(users::table)
            .select((Version::as_select(), Crate::as_select(), User::as_select()))
            .order(versions::created_at.desc())
            .limit(PER_PAGE)
            .offset(PER_PAGE * q.page.unwrap_or(0) as i64)
            .into_boxed();

        if let Some(q) = &q.q {
            // TODO: this is overly simplistic.
            query = query.filter(crates::name.ilike(format!("%{}%", q)));
        }

        let recent_versions = query
            .load::<(Version, Crate, User)>(conn)?
            .into_iter()
            .map(|(version, krate, user)| CrateVersion::new(version, krate, user));

        Ok(render_versions(&app.admin_engine, recent_versions))
    })
    .await
}
