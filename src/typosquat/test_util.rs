use diesel::{prelude::*, PgConnection};

use crate::{
    models::{
        Crate, CrateOwner, NewCrate, NewTeam, NewUser, NewVersion, Owner, OwnerKind, User, Version,
    },
    schema::{crate_downloads, crate_owners},
    Emails,
};

pub struct Faker {
    emails: Emails,
    id: i32,
}

impl Faker {
    pub fn new() -> Self {
        Self {
            emails: Emails::new_in_memory(),
            id: Default::default(),
        }
    }

    pub fn add_crate_to_team(
        &mut self,
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
        &mut self,
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

    pub fn team(
        &mut self,
        conn: &mut PgConnection,
        org: &str,
        team: &str,
    ) -> anyhow::Result<Owner> {
        Ok(Owner::Team(
            NewTeam::new(
                &format!("github:{org}:{team}"),
                self.next_id(),
                self.next_id(),
                Some(team.to_string()),
                None,
            )
            .create_or_update(conn)?,
        ))
    }

    pub fn user(&mut self, conn: &mut PgConnection, login: &str) -> anyhow::Result<User> {
        Ok(
            NewUser::new(self.next_id(), login, None, None, "token").create_or_update(
                None,
                &self.emails,
                conn,
            )?,
        )
    }

    fn next_id(&mut self) -> i32 {
        self.id += 1;
        self.id
    }
}
