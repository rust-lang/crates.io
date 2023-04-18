use super::prelude::*;
use axum::extract::Query;
use diesel::sql_types::Text;
use diesel_full_text_search::{TsQuery, TsQueryExtensions};

use crate::{
    extractors::admin::AdminUser,
    models::{Crate, User, Version},
    views::admin::crates::render,
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

impl CrateQuery {
    fn query_string(&self) -> Option<&str> {
        match &self.q {
            Some(q) if !q.is_empty() => Some(q.as_str()),
            _ => None,
        }
    }
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
        use diesel::dsl::*;

        let conn = &mut *app.db_read()?;

        let mut query = versions::table
            .inner_join(crates::table)
            .inner_join(users::table)
            .order(versions::created_at.desc())
            .limit(PER_PAGE)
            .offset(PER_PAGE * q.page.unwrap_or(0) as i64)
            .select((Version::as_select(), Crate::as_select(), User::as_select()))
            .into_boxed();

        if let Some(q_string) = q.query_string() {
            // FIXME: this is stolen from the public search controller, and
            // should be refactored into a common helper.
            let q = sql::<TsQuery>("plainto_tsquery('english', ")
                .bind::<Text, _>(q_string)
                .sql(")");
            query = query
                .filter(
                    q.clone()
                        .matches(crates::textsearchable_index_col)
                        .or(Crate::loosly_matches_name(q_string)),
                )
                .select((Version::as_select(), Crate::as_select(), User::as_select()))
                .order(Crate::with_name(q_string).desc())
                .then_order_by(versions::created_at.desc());
        }

        let recent_versions = query.load::<(Version, Crate, User)>(conn)?.into_iter();
        Ok(render(&app.admin_engine, q.query_string(), recent_versions))
    })
    .await
}
