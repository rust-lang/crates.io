use crate::controllers::frontend_prelude::*;
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;

use crate::models::Team;
use crate::schema::teams;
use crate::views::EncodableTeam;

/// Handles the `GET /teams/:team_id` route.
pub async fn show_team(state: AppState, Path(name): Path<String>) -> AppResult<Json<Value>> {
    use self::teams::dsl::{login, teams};

    let conn = state.db_read().await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();
        let team: Team = teams.filter(login.eq(&name)).first(conn)?;
        Ok(Json(json!({ "team": EncodableTeam::from(team) })))
    })
    .await
}
