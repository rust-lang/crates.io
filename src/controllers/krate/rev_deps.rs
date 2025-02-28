use crate::app::AppState;
use crate::controllers::helpers::pagination::PaginationOptions;
use crate::controllers::krate::CratePath;
use crate::models::{CrateName, User, Version, VersionOwnerAction};
use crate::util::errors::AppResult;
use crate::views::{EncodableDependency, EncodableVersion};
use axum::Json;
use crates_io_database::schema::{crates, users, versions};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::request::Parts;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RevDepsResponse {
    /// The list of reverse dependencies of the crate.
    dependencies: Vec<EncodableDependency>,

    /// The versions referenced in the `dependencies` field.
    versions: Vec<EncodableVersion>,

    #[schema(inline)]
    meta: RevDepsMeta,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RevDepsMeta {
    #[schema(example = 32)]
    total: i64,
}

/// List reverse dependencies of a crate.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/reverse_dependencies",
    params(CratePath),
    tag = "crates",
    responses((status = 200, description = "Successful Response", body = inline(RevDepsResponse))),
)]
pub async fn list_reverse_dependencies(
    app: AppState,
    path: CratePath,
    req: Parts,
) -> AppResult<Json<RevDepsResponse>> {
    let mut conn = app.db_read().await?;

    let pagination_options = PaginationOptions::builder().gather(&req)?;

    let krate = path.load_crate(&mut conn).await?;

    let offset = pagination_options.offset().unwrap_or_default();
    let limit = pagination_options.per_page;
    let (rev_deps, total) = krate.reverse_dependencies(&mut conn, offset, limit).await?;

    let rev_deps: Vec<_> = rev_deps
        .into_iter()
        .map(|dep| EncodableDependency::from_reverse_dep(dep, &krate.name))
        .collect();

    let version_ids: Vec<i32> = rev_deps.iter().map(|dep| dep.version_id).collect();

    let versions_and_publishers: Vec<(Version, CrateName, Option<User>)> = versions::table
        .filter(versions::id.eq_any(version_ids))
        .inner_join(crates::table)
        .left_outer_join(users::table)
        .select(<(Version, CrateName, Option<User>)>::as_select())
        .load(&mut conn)
        .await?;

    let versions = versions_and_publishers
        .iter()
        .map(|(v, ..)| v)
        .collect::<Vec<_>>();

    let actions = VersionOwnerAction::for_versions(&mut conn, &versions).await?;

    let versions = versions_and_publishers
        .into_iter()
        .zip(actions)
        .map(|((version, krate_name, published_by), actions)| {
            EncodableVersion::from(version, &krate_name.name, published_by, actions)
        })
        .collect::<Vec<_>>();

    Ok(Json(RevDepsResponse {
        dependencies: rev_deps,
        versions,
        meta: RevDepsMeta { total },
    }))
}
