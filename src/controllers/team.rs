use crate::controllers::frontend_prelude::*;

use crate::models::Team;
use crate::schema::teams;
use crate::views::EncodableTeam;

/// Handles the `GET /teams/:team_id` route.
pub fn show_team(req: ConduitRequest) -> AppResult<Response> {
    use self::teams::dsl::{login, teams};

    let name = req.param("team_id").unwrap();
    let conn = req.app().db_read()?;
    let team: Team = teams.filter(login.eq(name)).first(&*conn)?;

    Ok(req.json(json!({ "team": EncodableTeam::from(team) })))
}
