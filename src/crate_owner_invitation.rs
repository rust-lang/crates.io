use conduit::{Request, Response};
use diesel::prelude::*;
use time::Timespec;

use db::RequestTransaction;
use schema::{crate_owner_invitations, users, crates};
use user::RequestUser;
use util::errors::CargoResult;
use util::RequestUtils;

/// The model representing a row in the `crate_owner_invitations` database table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Identifiable, Queryable)]
#[primary_key(invited_user_id, crate_id)]
pub struct CrateOwnerInvitation {
    pub invited_user_id: i32,
    pub invited_by_user_id: i32,
    pub crate_id: i32,
    pub created_at: Timespec,
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
            created_at: ::encode_time(self.created_at),
        }
    }
}

/// The serialization format for the `CrateOwnerInvitation` model.
#[derive(Deserialize, Serialize, Debug)]
pub struct EncodableCrateOwnerInvitation {
    pub invited_by_username: String,
    pub crate_name: String,
    pub crate_id: i32,
    pub created_at: String,
}

/// Handles the `GET /me/crate_owner_invitations` route.
pub fn list(req: &mut Request) -> CargoResult<Response> {
    let conn = &*req.db_conn()?;
    let user_id = req.user()?.id;

    let crate_owner_invitations = crate_owner_invitations::table
        .filter(crate_owner_invitations::invited_user_id.eq(user_id))
        .load::<CrateOwnerInvitation>(&*conn)?
        .into_iter()
        .map(|i| i.encodable(conn))
        .collect();

    #[derive(Serialize)]
    struct R {
        crate_owner_invitations: Vec<EncodableCrateOwnerInvitation>,
    }
    Ok(req.json(&R { crate_owner_invitations }))
}
