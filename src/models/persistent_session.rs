use chrono::NaiveDateTime;
use diesel::prelude::*;
use ipnetwork::IpNetwork;
use std::net::IpAddr;
use thiserror::Error;

use crate::schema::persistent_sessions;
use crate::util::token::SecureToken;
use crate::util::token::SecureTokenKind;

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

#[derive(Error, Debug, PartialEq)]
pub enum SessionError {
    #[error("token prefix doesn't match session tokens")]
    InvalidSessionToken,
    #[error("database manipulation error")]
    DieselError(#[from] diesel::result::Error),
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

    pub fn find_from_token_and_update(
        conn: &PgConnection,
        token: &str,
        ip_address: IpAddr,
        user_agent: &str,
    ) -> Result<Option<Self>, SessionError> {
        let hashed_token = SecureToken::parse(SecureTokenKind::Session, token)
            .ok_or(SessionError::InvalidSessionToken)?;
        let sessions = persistent_sessions::table
            .filter(persistent_sessions::revoked.eq(false))
            .filter(persistent_sessions::hashed_token.eq(hashed_token));

        conn.transaction(|| {
            diesel::update(sessions.clone())
                .set((
                    persistent_sessions::last_used_at.eq(diesel::dsl::now),
                    persistent_sessions::last_ip_address.eq(IpNetwork::from(ip_address)),
                    persistent_sessions::last_user_agent.eq(user_agent),
                ))
                .get_result(conn)
                .optional()
        })
        .or_else(|_| sessions.first(conn).optional())
        .map_err(SessionError::DieselError)
    }

    pub fn revoke_from_token(conn: &PgConnection, token: &str) -> Result<usize, SessionError> {
        let hashed_token = SecureToken::parse(SecureTokenKind::Session, token)
            .ok_or(SessionError::InvalidSessionToken)?;
        let sessions = persistent_sessions::table
            .filter(persistent_sessions::hashed_token.eq(hashed_token))
            .filter(persistent_sessions::revoked.eq(false));

        diesel::update(sessions)
            .set(persistent_sessions::revoked.eq(true))
            .execute(conn)
            .map_err(SessionError::DieselError)
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
