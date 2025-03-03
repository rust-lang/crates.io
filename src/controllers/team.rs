use crate::app::AppState;
use crate::models::Team;
use crate::util::errors::AppResult;
use crate::views::EncodableTeam;
use axum::Json;
use axum::extract::Path;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct GetResponse {
    team: EncodableTeam,
}

/// Find team by login.
#[utoipa::path(
    get,
    path = "/api/v1/teams/{team}",
    params(
        ("team" = String, Path, description = "Name of the team", example = "github:rust-lang:crates-io"),
    ),
    tag = "teams",
    responses((status = 200, description = "Successful Response", body = inline(GetResponse))),
)]
pub async fn find_team(state: AppState, Path(name): Path<String>) -> AppResult<Json<GetResponse>> {
    use crate::schema::teams::dsl::{login, teams};

    let mut conn = state.db_read().await?;
    let team: Team = teams.filter(login.eq(&name)).first(&mut conn).await?;
    let team = EncodableTeam::from(team);
    Ok(Json(GetResponse { team }))
}
