use bon::Builder;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::models::{Crate, CrateOwner, Owner, OwnerKind};
use crate::schema::{crate_owners, teams};

/// For now, just a GitHub Team. Can be upgraded to other teams
/// later if desirable.
#[derive(Queryable, Identifiable, serde::Serialize, serde::Deserialize, Debug, Selectable)]
pub struct Team {
    /// Unique table id
    pub id: i32,
    /// "github:org:team"
    /// An opaque unique ID, that was at one point parsed out to query GitHub.
    /// We only query membership with github using the github_id, though.
    /// This is the only name we should ever talk to Cargo about.
    pub login: String,
    /// The GitHub API works on team ID numbers. This can change, if a team
    /// is deleted and then recreated with the same name!!!
    pub github_id: i32,
    /// Sugary goodness
    pub name: Option<String>,
    pub avatar: Option<String>,
    /// The GitHub Organization ID this team sits under
    pub org_id: i32,
}

#[derive(Insertable, AsChangeset, Debug, Builder)]
#[diesel(table_name = teams, check_for_backend(diesel::pg::Pg))]
pub struct NewTeam<'a> {
    pub login: &'a str,
    pub github_id: i32,
    pub name: Option<&'a str>,
    pub avatar: Option<&'a str>,
    pub org_id: i32,
}

impl NewTeam<'_> {
    pub async fn create_or_update(&self, conn: &mut AsyncPgConnection) -> QueryResult<Team> {
        use diesel::insert_into;

        insert_into(teams::table)
            .values(self)
            .on_conflict(teams::github_id)
            .do_update()
            .set(self)
            .get_result(conn)
            .await
    }
}

impl Team {
    pub async fn owning(krate: &Crate, conn: &mut AsyncPgConnection) -> QueryResult<Vec<Owner>> {
        let base_query = CrateOwner::belonging_to(krate).filter(crate_owners::deleted.eq(false));
        let teams = base_query
            .inner_join(teams::table)
            .select(Team::as_select())
            .filter(crate_owners::owner_kind.eq(OwnerKind::Team))
            .load(conn)
            .await?
            .into_iter()
            .map(Owner::Team);

        Ok(teams.collect())
    }
}
