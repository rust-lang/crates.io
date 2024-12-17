use crate::app::AppState;
use crate::controllers::helpers::pagination::PaginationOptions;
use crate::controllers::krate::CratePath;
use crate::models::{CrateName, User, Version, VersionOwnerAction};
use crate::util::errors::AppResult;
use crate::views::{EncodableDependency, EncodableVersion};
use axum_extra::json;
use axum_extra::response::ErasedJson;
use crates_io_database::schema::{crates, users, versions};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::request::Parts;

/// List reverse dependencies of a crate.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/reverse_dependencies",
    params(CratePath),
    tag = "crates",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn list_reverse_dependencies(
    app: AppState,
    path: CratePath,
    req: Parts,
) -> AppResult<ErasedJson> {
    let mut conn = app.db_read().await?;

    let pagination_options = PaginationOptions::builder().gather(&req)?;

    let krate = path.load_crate(&mut conn).await?;

    let (rev_deps, total) = krate
        .reverse_dependencies(&mut conn, pagination_options)
        .await?;

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

    Ok(json!({
        "dependencies": rev_deps,
        "versions": versions,
        "meta": { "total": total },
    }))
}
