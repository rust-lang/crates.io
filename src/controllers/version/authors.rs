use crate::controllers::version::CrateVersionPath;
use axum_extra::json;
use axum_extra::response::ErasedJson;

/// Get crate version authors.
///
/// This endpoint was deprecated by [RFC #3052](https://github.com/rust-lang/rfcs/pull/3052)
/// and returns an empty list for backwards compatibility reasons.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/{version}/authors",
    params(CrateVersionPath),
    tag = "versions",
    responses((status = 200, description = "Successful Response")),
)]
#[deprecated]
pub async fn get_version_authors() -> ErasedJson {
    json!({
        "users": [],
        "meta": { "names": [] },
    })
}
