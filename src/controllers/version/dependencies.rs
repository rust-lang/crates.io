use super::CrateVersionPath;
use crate::models::Dependency;
use crate::server::ServerContext;
use crate::util::errors::AppResult;
use crate::views::EncodableDependency;
use axum::Json;
use crates_io_database::schema::{crates, dependencies};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Serialize;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct Response {
    pub dependencies: Vec<EncodableDependency>,
}

/// Get crate version dependencies.
///
/// This information can also be obtained directly from the index.
///
/// In addition to returning cached data from the index, this returns
/// fields for `id`, `version_id`, and `downloads` (which appears to always
/// be 0)
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/{version}/dependencies",
    params(CrateVersionPath),
    tag = "versions",
    responses(
        (status = 200, description = "Successful Response", body = inline(Response)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn get_version_dependencies(
    ctx: ServerContext,
    path: CrateVersionPath,
) -> AppResult<Json<Response>> {
    let mut conn = ctx.db_read().await?;
    let version = path.load_version(&conn).await?;

    let dependencies = Dependency::belonging_to(&version)
        .inner_join(crates::table)
        .select((Dependency::as_select(), crates::name))
        .order((dependencies::optional, crates::name))
        .load::<(Dependency, String)>(&mut conn)
        .await?
        .into_iter()
        .map(|(dep, crate_name)| EncodableDependency::from_dep(dep, &crate_name))
        .collect::<Vec<_>>();

    Ok(Json(Response { dependencies }))
}
