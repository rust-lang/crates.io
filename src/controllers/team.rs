use crate::controllers::frontend_prelude::*;

use crate::models::Team;
use crate::schema::teams;
use crate::views::EncodableTeam;

/// Handles the `GET /teams/:team_id` route.
pub fn show_team(req: &mut dyn RequestExt) -> EndpointResult {
    use self::teams::dsl::{login, teams};

    let name = &req.params()["team_id"];
    let conn = req.db_read_only()?;
    let team: Team = teams.filter(login.eq(name)).first(&*conn)?;

    Ok(req.json(&json!({ "team": EncodableTeam::from(team) })))
}
