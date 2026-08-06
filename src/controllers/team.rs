use crate::app::AppState;
use crate::models::Team;
use crate::util::errors::AppResult;
use crate::views::EncodableTeam;
use axum::Json;
use axum::extract::Path;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Serialize;

/// Response returned when getting a team by login.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TeamGetResponse {
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
    responses(
        (status = 200, description = "Successful Response", body = inline(TeamGetResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn find_team(
    state: AppState,
    Path(name): Path<String>,
) -> AppResult<Json<TeamGetResponse>> {
    use crate::schema::teams::dsl::login;

    let mut conn = state.db_read().await?;
    let team: Team = Team::query()
        .filter(login.eq(&name))
        .first(&mut conn)
        .await?;
    let team = EncodableTeam::from(team);
    Ok(Json(TeamGetResponse { team }))
}
