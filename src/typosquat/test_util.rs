use diesel::{prelude::*, PgConnection};

use crate::tests::util::github::next_gh_id;
use crate::{
    models::{Crate, CrateOwner, NewTeam, NewUser, OwnerKind, Team, User},
    schema::{crate_owners, users},
};

pub mod faker {
    use super::*;
    use crate::tests::builders::CrateBuilder;
    use anyhow::anyhow;

    pub fn add_crate_to_team(
        conn: &mut PgConnection,
        user: &User,
        krate: &Crate,
        team: &Team,
    ) -> anyhow::Result<()> {
        // We have to do a bunch of this by hand, since normally adding a team owner triggers
        // various checks.
        diesel::insert_into(crate_owners::table)
            .values(&CrateOwner {
                crate_id: krate.id,
                owner_id: team.id,
                created_by: user.id,
                owner_kind: OwnerKind::Team,
                email_notifications: true,
            })
            .execute(conn)?;

        Ok(())
    }

    pub fn crate_and_version(
        conn: &mut PgConnection,
        name: &str,
        description: &str,
        user: &User,
        downloads: i32,
    ) -> anyhow::Result<Crate> {
        CrateBuilder::new(name, user.id)
            .description(description)
            .downloads(downloads)
            .version("1.0.0")
            .build(conn)
            .map_err(|err| anyhow!(err.to_string()))
    }

    pub fn team(conn: &mut PgConnection, org: &str, team: &str) -> anyhow::Result<Team> {
        let login = format!("github:{org}:{team}");
        let team = NewTeam::builder()
            .login(&login)
            .org_id(next_gh_id())
            .github_id(next_gh_id())
            .name(team)
            .build();

        Ok(team.create_or_update(conn)?)
    }

    pub fn user(conn: &mut PgConnection, login: &str) -> QueryResult<User> {
        let user = NewUser::new(next_gh_id(), login, None, None, "token");

        diesel::insert_into(users::table)
            .values(user)
            .get_result(conn)
    }
}
