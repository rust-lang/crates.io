use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::schema::{crate_owner_invitations, crates, users};
use crate::views::EncodableCrateOwnerInvitation;

/// The model representing a row in the `crate_owner_invitations` database table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Identifiable, Queryable)]
#[primary_key(invited_user_id, crate_id)]
pub struct CrateOwnerInvitation {
    pub invited_user_id: i32,
    pub invited_by_user_id: i32,
    pub crate_id: i32,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable, Clone, Copy, Debug)]
#[table_name = "crate_owner_invitations"]
pub struct NewCrateOwnerInvitation {
    pub invited_user_id: i32,
    pub invited_by_user_id: i32,
    pub crate_id: i32,
}

impl CrateOwnerInvitation {
    pub fn invited_by_username(&self, conn: &PgConnection) -> String {
        users::table
            .find(self.invited_by_user_id)
            .select(users::gh_login)
            .first(&*conn)
            .unwrap_or_else(|_| String::from("(unknown username)"))
    }

    pub fn crate_name(&self, conn: &PgConnection) -> String {
        crates::table
            .find(self.crate_id)
            .select(crates::name)
            .first(&*conn)
            .unwrap_or_else(|_| String::from("(unknown crate name)"))
    }

    pub fn encodable(self, conn: &PgConnection) -> EncodableCrateOwnerInvitation {
        EncodableCrateOwnerInvitation {
            invited_by_username: self.invited_by_username(conn),
            crate_name: self.crate_name(conn),
            crate_id: self.crate_id,
            created_at: self.created_at,
        }
    }
}
