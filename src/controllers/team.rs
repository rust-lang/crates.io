use crate::controllers::prelude::*;

use crate::models::Team;
use crate::schema::teams;
use crate::views::EncodableTeam;

/// Handles the `GET /teams/:team_id` route.
pub fn show_team(req: &mut dyn Request) -> CargoResult<Response> {
    use self::teams::dsl::{login, teams};

    let name = &req.params()["team_id"];
    let conn = req.db_conn()?;
    let team = teams.filter(login.eq(name)).first::<Team>(&*conn)?;

    #[derive(Serialize)]
    struct R {
        team: EncodableTeam,
    }
    Ok(req.json(&R {
        team: team.encodable(),
    }))
}
