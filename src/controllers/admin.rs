use axum::extract::Query;
use axum::response::IntoResponse;
use diesel::sql_types::Text;
use diesel_full_text_search::TsQuery;
use diesel_full_text_search::TsQueryExtensions;

use crate::auth::AuthCheck;
use crate::controllers::frontend_prelude::*;
use crate::controllers::helpers::pagination::Page;
use crate::controllers::helpers::pagination::Paginated;
use crate::controllers::helpers::pagination::PaginationOptions;
use crate::controllers::helpers::*;
use crate::models::Crate;
use crate::models::User;
use crate::models::Version;
use crate::util::errors::AppResult;
use crate::views;

/// Handles the `GET /admin/` route.
pub async fn index(app: AppState, req: Parts) -> impl IntoResponse {
    conduit_compat(move || {
        let conn = &mut *app.db_read()?;
        let _auth = AuthCheck::default().require_admin().check(&req, conn)?;

        Ok(redirect("/admin/crates/".to_string()))
    })
    .await
}

#[derive(Deserialize)]
pub struct CrateQuery {
    q: Option<String>,
    page: Option<u32>,
}

impl CrateQuery {
    fn page(&self) -> u32 {
        self.page.unwrap_or(1)
    }

    fn query_string(&self) -> Option<&str> {
        match &self.q {
            Some(q) if !q.is_empty() => Some(q.as_str()),
            _ => None,
        }
    }
}

/// Handles the `GET /admin/crates/` route.
pub async fn crates(app: AppState, q: Query<CrateQuery>) -> AppResult<impl IntoResponse> {
    const PER_PAGE: u32 = 50;

    let pagination = PaginationOptions {
        page: Page::Numeric(q.page()),
        per_page: PER_PAGE as i64,
    };

    conduit_compat(move || {
        use crate::schema::{crates, users, versions};
        use diesel::dsl::*;

        let conn = &mut *app.db_read()?;

        let mut query = versions::table
            .inner_join(crates::table)
            .inner_join(users::table)
            .order(versions::created_at.desc())
            .select((
                versions::all_columns,
                crate::models::krate::ALL_COLUMNS,
                users::all_columns,
            ))
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
                .order(Crate::with_name(q_string).desc())
                .then_order_by(versions::created_at.desc());
        }

        let data: Paginated<(Version, Crate, User)> =
            query.pages_pagination(pagination).load(conn)?;
        Ok(views::admin::crates::render(
            &app.admin_engine,
            q.query_string(),
            data,
        ))
    })
    .await
}
