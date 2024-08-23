use crate::app::AppState;
use crate::models::Team;
use crate::util::errors::AppResult;
use crate::views::EncodableTeam;
use axum::extract::Path;
use axum::Json;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::Value;

/// Handles the `GET /teams/:team_id` route.
pub async fn show_team(state: AppState, Path(name): Path<String>) -> AppResult<Json<Value>> {
    use crate::schema::teams::dsl::{login, teams};

    let mut conn = state.db_read().await?;
    let team: Team = teams.filter(login.eq(&name)).first(&mut conn).await?;
    Ok(Json(json!({ "team": EncodableTeam::from(team) })))
}
