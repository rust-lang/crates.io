use crate::controllers::frontend_prelude::*;

use crate::models::Team;
use crate::schema::teams;
use crate::views::EncodableTeam;

/// Handles the `GET /teams/:team_id` route.
pub async fn show_team(Path(name): Path<String>, req: Parts) -> AppResult<Json<Value>> {
    conduit_compat(move || {
        use self::teams::dsl::{login, teams};

        let conn = req.app().db_read()?;
        let team: Team = teams.filter(login.eq(&name)).first(&*conn)?;

        Ok(Json(json!({ "team": EncodableTeam::from(team) })))
    })
    .await
}
