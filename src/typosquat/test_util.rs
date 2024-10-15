use diesel::{prelude::*, PgConnection};

use crate::tests::util::github::next_gh_id;
use crate::{
    models::{
        Crate, CrateOwner, NewCrate, NewTeam, NewUser, NewVersion, Owner, OwnerKind, User, Version,
    },
    schema::{crate_downloads, crate_owners, users},
};

pub mod faker {
    use super::*;

    pub fn add_crate_to_team(
        conn: &mut PgConnection,
        user: &User,
        krate: &Crate,
        team: &Owner,
    ) -> anyhow::Result<()> {
        // We have to do a bunch of this by hand, since normally adding a team owner triggers
        // various checks.
        diesel::insert_into(crate_owners::table)
            .values(&CrateOwner {
                crate_id: krate.id,
                owner_id: team.id(),
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
    ) -> anyhow::Result<(Crate, Version)> {
        let krate = NewCrate {
            name,
            description: Some(description),
            ..Default::default()
        }
        .create(conn, user.id)?;

        diesel::update(crate_downloads::table)
            .filter(crate_downloads::crate_id.eq(krate.id))
            .set(crate_downloads::downloads.eq(downloads as i64))
            .execute(conn)?;

        let version = NewVersion::builder(krate.id, "1.0.0")
            .published_by(user.id)
            .dummy_checksum()
            .build()
            .unwrap()
            .save(conn, "someone@example.com")
            .unwrap();

        Ok((krate, version))
    }

    pub fn team(conn: &mut PgConnection, org: &str, team: &str) -> anyhow::Result<Owner> {
        Ok(Owner::Team(
            NewTeam::new(
                &format!("github:{org}:{team}"),
                next_gh_id(),
                next_gh_id(),
                Some(team.to_string()),
                None,
            )
            .create_or_update(conn)?,
        ))
    }

    pub fn user(conn: &mut PgConnection, login: &str) -> QueryResult<User> {
        let user = NewUser::new(next_gh_id(), login, None, None, "token");

        diesel::insert_into(users::table)
            .values(user)
            .get_result(conn)
    }
}
