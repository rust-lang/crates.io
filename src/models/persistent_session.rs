use chrono::NaiveDateTime;
use diesel::prelude::*;
use ipnetwork::IpNetwork;
use std::net::IpAddr;
use thiserror::Error;

use crate::schema::persistent_sessions;
use crate::util::token::SecureToken;
use crate::util::token::SecureTokenKind;

/// A persistent session model (as is stored in the database).
///
/// The sessions table works by maintaining a `hashed_token`. In order for a user to securely
/// demonstrate authenticity, the user provides us with the token (stored as part of a cookie). We
/// hash the token and search in the database for matches. If we find one and the token hasn't
/// been revoked, then we update the session with the latest values and authorize the user.
#[derive(Clone, Debug, PartialEq, Eq, Identifiable, Queryable)]
#[table_name = "persistent_sessions"]
pub struct PersistentSession {
    /// The id of this session.
    pub id: i32,
    /// The user id associated with this session.
    pub user_id: i32,
    /// The token (hashed) that identifies the session.
    pub hashed_token: SecureToken,
    /// Datetime the session was created.
    pub created_at: NaiveDateTime,
    /// Datetime the session was last used.
    pub last_used_at: NaiveDateTime,
    /// Whether the session is revoked.
    pub revoked: bool,
    /// Last IP address that used the session.
    pub last_ip_address: IpNetwork,
    /// Last user agent that used the session.
    pub last_user_agent: String,
}

/// Session-related errors.
#[derive(Error, Debug, PartialEq)]
pub enum SessionError {
    #[error("token prefix doesn't match session tokens")]
    InvalidSessionToken,
    #[error("database manipulation error")]
    DieselError(#[from] diesel::result::Error),
}

impl PersistentSession {
    /// Creates a `NewPersistentSession` that can be inserted into the database.
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

    /// Finds an unrevoked session that matches `token` from the database and returns it.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(...))` if a session matches the `token`.
    /// * `Ok(None)` if no session matches the `token`.
    /// * `Err(...)` for other errors such as invalid tokens or diesel errors.
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

        // TODO: Do we want to check if the user agent or IP address don't match? What about the
        // created_at/last_user_at times, do we want to expire the tokens?
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

    /// Revokes the `token` in the database.
    ///
    /// # Returns
    ///
    /// The number of sessions that were revoked or an error if the `token` isn't valid or there
    /// was an issue with the database connection.
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

/// A new, insertable persistent session.
#[derive(Clone, Debug, PartialEq, Eq, Insertable)]
#[table_name = "persistent_sessions"]
pub struct NewPersistentSession<'a, 'b> {
    user_id: i32,
    hashed_token: &'a SecureToken,
    last_ip_address: IpNetwork,
    last_user_agent: &'b str,
}

impl NewPersistentSession<'_, '_> {
    /// Inserts the session into the database.
    pub fn insert(self, conn: &PgConnection) -> Result<PersistentSession, diesel::result::Error> {
        diesel::insert_into(persistent_sessions::table)
            .values(self)
            .get_result(conn)
    }
}
