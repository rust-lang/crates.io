use crate::{
    app::AppState,
    auth::AuthCheck,
    models::{CrateOwner, OwnerKind, User, Version},
    schema::*,
    util::errors::{AppResult, custom},
};
use axum::{Json, extract::Path};
use chrono::{DateTime, Utc};
use diesel::{dsl::count_star, prelude::*};
use diesel_async::RunQueryDsl;
use http::{StatusCode, request::Parts};
use serde::Serialize;
use std::collections::HashMap;

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

    let (user, verified, email) = users::table
        .left_join(emails::table)
        .filter(users::gh_login.eq(username))
        .select((
            User::as_select(),
            emails::verified.nullable(),
            emails::email.nullable(),
        ))
        .first::<(User, Option<bool>, Option<String>)>(&mut conn)
        .await?;

    let crates: Vec<(i32, String, Option<String>, DateTime<Utc>, i64)> =
        CrateOwner::by_owner_kind(OwnerKind::User)
            .inner_join(crates::table)
            .filter(crate_owners::owner_id.eq(user.id))
            .select((
                crates::id,
                crates::name,
                crates::description,
                crates::updated_at,
                rev_deps_subquery(),
            ))
            .order(crates::name.asc())
            .load(&mut conn)
            .await?;

    let crate_ids: Vec<_> = crates.iter().map(|(id, ..)| id).collect();

    let versions: Vec<Version> = versions::table
        .filter(versions::crate_id.eq_any(crate_ids))
        .select(Version::as_select())
        .load(&mut conn)
        .await?;
    let mut versions_by_crate_id: HashMap<i32, Vec<Version>> = HashMap::new();
    for version in versions {
        let crate_versions = versions_by_crate_id.entry(version.crate_id).or_default();
        crate_versions.push(version);
    }

    let verified = verified.unwrap_or(false);
    let crates = crates
        .into_iter()
        .map(|(crate_id, name, description, updated_at, num_rev_deps)| {
            let versions = versions_by_crate_id.get(&crate_id);
            let last_version = versions.and_then(|v| v.last());
            AdminCrateInfo {
                name,
                description,
                updated_at,
                num_rev_deps,
                num_versions: versions.map(|v| v.len()).unwrap_or(0),
                crate_size: last_version.map(|v| v.crate_size).unwrap_or(0),
                bin_names: last_version
                    .map(|v| v.bin_names.clone())
                    .unwrap_or_default(),
            }
        })
        .collect();
    Ok(Json(AdminListResponse {
        user_email: verified.then_some(email).flatten(),
        crates,
    }))
}

#[derive(Debug, Serialize)]
pub struct AdminListResponse {
    user_email: Option<String>,
    crates: Vec<AdminCrateInfo>,
}

#[derive(Debug, Serialize)]
pub struct AdminCrateInfo {
    pub name: String,
    pub description: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub num_rev_deps: i64,
    pub num_versions: usize,
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
