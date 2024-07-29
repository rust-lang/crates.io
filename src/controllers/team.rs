use crate::controllers::frontend_prelude::*;

use crate::models::Team;
use crate::schema::teams;
use crate::views::EncodableTeam;
use diesel_async::RunQueryDsl;

/// Handles the `GET /teams/:team_id` route.
pub async fn show_team(state: AppState, Path(name): Path<String>) -> AppResult<Json<Value>> {
    use self::teams::dsl::{login, teams};

    let mut conn = state.db_read().await?;
    let team: Team = teams.filter(login.eq(&name)).first(&mut conn).await?;
    Ok(Json(json!({ "team": EncodableTeam::from(team) })))
}
