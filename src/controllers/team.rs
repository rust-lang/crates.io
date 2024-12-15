use crate::app::AppState;
use crate::models::Team;
use crate::util::errors::AppResult;
use crate::views::EncodableTeam;
use axum::extract::Path;
use axum_extra::json;
use axum_extra::response::ErasedJson;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

/// Find team by login.
#[utoipa::path(
    get,
    path = "/api/v1/teams/{team}",
    tag = "teams",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn find_team(state: AppState, Path(name): Path<String>) -> AppResult<ErasedJson> {
    use crate::schema::teams::dsl::{login, teams};

    let mut conn = state.db_read().await?;
    let team: Team = teams.filter(login.eq(&name)).first(&mut conn).await?;
    Ok(json!({ "team": EncodableTeam::from(team) }))
}
