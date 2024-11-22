use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::tests::util::github::next_gh_id;
use crate::{
    models::{Crate, NewTeam, NewUser, Team, User},
    schema::users,
};

pub mod faker {
    use super::*;
    use crate::tests::builders::CrateBuilder;
    use anyhow::anyhow;
    use diesel_async::AsyncPgConnection;

    pub async fn crate_and_version(
        conn: &mut AsyncPgConnection,
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
            .await
            .map_err(|err| anyhow!(err.to_string()))
    }

    pub async fn team(conn: &mut AsyncPgConnection, org: &str, team: &str) -> anyhow::Result<Team> {
        let login = format!("github:{org}:{team}");
        let team = NewTeam::builder()
            .login(&login)
            .org_id(next_gh_id())
            .github_id(next_gh_id())
            .name(team)
            .build();

        Ok(team.async_create_or_update(conn).await?)
    }

    pub async fn user(conn: &mut AsyncPgConnection, login: &str) -> QueryResult<User> {
        let user = NewUser::new(next_gh_id(), login, None, None, "token");

        diesel::insert_into(users::table)
            .values(user)
            .get_result(conn)
            .await
    }
}
