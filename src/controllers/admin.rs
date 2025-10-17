use crate::{
    app::AppState,
    auth::AuthCheck,
    models::{OwnerKind, User},
    schema::*,
    util::errors::{AppResult, custom},
};
use axum::{Json, extract::Path};
use chrono::{DateTime, Utc};
use diesel::{dsl::count_star, prelude::*};
use diesel_async::RunQueryDsl;
use http::{StatusCode, request::Parts};
use serde::Serialize;

#[derive(Debug, HasQuery)]
#[diesel(
    base_query = crate_owners::table
        .inner_join(crates::table)
        .left_join(crate_downloads::table.on(crates::id.eq(crate_downloads::crate_id)))
        .left_join(
            recent_crate_downloads::table.on(crates::id.eq(recent_crate_downloads::crate_id)),
        )
        .inner_join(default_versions::table.on(crates::id.eq(default_versions::crate_id)))
        .inner_join(versions::table.on(default_versions::version_id.eq(versions::id)))
)]
struct DatabaseCrateInfo {
    #[diesel(select_expression = crates::columns::name)]
    name: String,

    #[diesel(select_expression = crates::columns::description)]
    description: Option<String>,

    #[diesel(select_expression = crates::columns::updated_at)]
    updated_at: DateTime<Utc>,

    #[diesel(select_expression = crate_downloads::columns::downloads.nullable())]
    downloads: Option<i64>,

    #[diesel(select_expression = recent_crate_downloads::columns::downloads.nullable())]
    recent_crate_downloads: Option<i64>,

    #[diesel(select_expression = default_versions::columns::num_versions)]
    num_versions: Option<i32>,

    #[diesel(select_expression = versions::columns::yanked)]
    yanked: bool,

    #[diesel(select_expression = versions::columns::num)]
    default_version_num: String,

    #[diesel(select_expression = versions::columns::crate_size)]
    crate_size: i32,

    #[diesel(select_expression = versions::columns::bin_names)]
    bin_names: Option<Vec<Option<String>>>,

    #[diesel(select_expression = rev_deps_subquery())]
    num_rev_deps: i64,
}

/// Handles the `GET /api/private/admin_list/{username}` endpoint.
pub async fn list(
    state: AppState,
    Path(username): Path<String>,
    req: Parts,
) -> AppResult<Json<AdminListResponse>> {
    let mut conn = state.db_read().await?;

    let auth = AuthCheck::default().check(&req, &mut conn).await?;
    let logged_in_user = auth.user();

    if !logged_in_user.is_admin {
        return Err(custom(
            StatusCode::FORBIDDEN,
            "must be an admin to use this route",
        ));
    }

    let (user, verified, user_email) = users::table
        .left_join(emails::table)
        .filter(users::gh_login.eq(username))
        .select((
            User::as_select(),
            emails::verified.nullable(),
            emails::email.nullable(),
        ))
        .first::<(User, Option<bool>, Option<String>)>(&mut conn)
        .await?;

    let crates: Vec<DatabaseCrateInfo> = DatabaseCrateInfo::query()
        .filter(crate_owners::deleted.eq(false))
        .filter(crate_owners::owner_kind.eq(OwnerKind::User))
        .filter(crate_owners::owner_id.eq(user.id))
        .order(crates::name.asc())
        .load(&mut conn)
        .await?;

    let crates = crates
        .into_iter()
        .map(|database_crate_info| {
            let DatabaseCrateInfo {
                name,
                description,
                updated_at,
                downloads,
                recent_crate_downloads,
                num_versions,
                yanked,
                default_version_num,
                crate_size,
                bin_names,
                num_rev_deps,
            } = database_crate_info;

            AdminCrateInfo {
                name,
                description,
                updated_at,
                downloads: downloads.unwrap_or_default()
                    + recent_crate_downloads.unwrap_or_default(),
                num_rev_deps,
                num_versions: num_versions.unwrap_or_default() as usize,
                yanked,
                default_version_num,
                crate_size,
                bin_names,
            }
        })
        .collect();
    Ok(Json(AdminListResponse {
        user_email,
        user_email_verified: verified.unwrap_or_default(),
        crates,
    }))
}

#[derive(Debug, Serialize)]
pub struct AdminListResponse {
    user_email: Option<String>,
    user_email_verified: bool,
    crates: Vec<AdminCrateInfo>,
}

#[derive(Debug, Serialize)]
pub struct AdminCrateInfo {
    pub name: String,
    pub description: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub downloads: i64,
    pub num_rev_deps: i64,
    pub num_versions: usize,
    pub yanked: bool,
    pub default_version_num: String,
    pub crate_size: i32,
    pub bin_names: Option<Vec<Option<String>>>,
}

/// A subquery that returns the number of reverse dependencies of a crate.
///
/// **Warning:** this is an incorrect reverse dependencies query, since it
/// includes the `dependencies` rows for all versions, not just the
/// "default version" per crate. However, it's good enough for our
/// purposes here.
#[diesel::dsl::auto_type]
fn rev_deps_subquery() -> _ {
    dependencies::table
        .select(count_star())
        .filter(dependencies::crate_id.eq(crates::id))
        .single_value()
        .assume_not_null()
}
