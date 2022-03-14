use chrono::NaiveDateTime;
use diesel::prelude::*;
use ipnetwork::IpNetwork;
use std::net::IpAddr;

use crate::schema::persistent_sessions;
use crate::util::token::SecureToken;

#[derive(Clone, Debug, PartialEq, Eq, Identifiable, Queryable)]
#[table_name = "persistent_sessions"]
pub struct PersistentSession {
    pub id: i32,
    pub user_id: i32,
    pub hashed_token: SecureToken,
    pub created_at: NaiveDateTime,
    pub last_used_at: NaiveDateTime,
    pub revoked: bool,
    pub last_ip_address: IpNetwork,
    pub last_user_agent: String,
}

impl PersistentSession {
    pub fn create<'a, 'b>(
        user_id: i32,
        token: &'a SecureToken,
        last_ip_address: IpAddr,
        last_user_agent: &'b str,
    ) -> NewPersistentSession<'a, 'b> {
        NewPersistentSession {
            user_id,
            hashed_token: token,
            last_ip_address: last_ip_address.into(),
            last_user_agent,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Insertable)]
#[table_name = "persistent_sessions"]
pub struct NewPersistentSession<'a, 'b> {
    user_id: i32,
    hashed_token: &'a SecureToken,
    last_ip_address: IpNetwork,
    last_user_agent: &'b str,
}

impl NewPersistentSession<'_, '_> {
    pub fn insert(self, conn: &PgConnection) -> Result<PersistentSession, diesel::result::Error> {
        diesel::insert_into(persistent_sessions::table)
            .values(self)
            .get_result(conn)
    }
}
