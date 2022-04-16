use chrono::NaiveDateTime;
use cookie::{Cookie, SameSite};
use diesel::prelude::*;
use std::num::ParseIntError;
use std::str::FromStr;
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
    pub id: i64,
    /// The user id associated with this session.
    pub user_id: i32,
    /// The token (hashed) that identifies the session.
    pub hashed_token: SecureToken,
    /// Datetime the session was created.
    pub created_at: NaiveDateTime,
    /// Whether the session is revoked.
    pub revoked: bool,
}

impl PersistentSession {
    /// Creates a `NewPersistentSession` that can be inserted into the database.
    pub fn create<'a>(user_id: i32, token: &'a SecureToken) -> NewPersistentSession<'a> {
        NewPersistentSession {
            user_id,
            hashed_token: token,
        }
    }

    /// Finds the session with the ID.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(...))` if a session matches the id.
    /// * `Ok(None)` if no session matches the id.
    /// * `Err(...)` for other errors..
    pub fn find(id: i64, conn: &PgConnection) -> Result<Option<Self>, diesel::result::Error> {
        persistent_sessions::table
            .find(id)
            .get_result(conn)
            .optional()
    }

    /// Updates the session in the database.
    pub fn update(&self, conn: &PgConnection) -> Result<(), diesel::result::Error> {
        diesel::update(persistent_sessions::table.find(self.id))
            .set((
                persistent_sessions::user_id.eq(&self.user_id),
                persistent_sessions::hashed_token.eq(&self.hashed_token),
                persistent_sessions::revoked.eq(&self.revoked),
            ))
            .get_result::<Self>(conn)
            .map(|_| ())
    }

    pub fn is_authorized(&self, token: &str) -> bool {
        if let Some(hashed_token) = SecureToken::parse(SecureTokenKind::Session, token) {
            !self.revoked && self.hashed_token == hashed_token
        } else {
            false
        }
    }

    /// Revokes the session (needs update).
    pub fn revoke(&mut self) -> &mut Self {
        self.revoked = true;
        self
    }
}

/// A new, insertable persistent session.
#[derive(Clone, Debug, PartialEq, Eq, Insertable)]
#[table_name = "persistent_sessions"]
pub struct NewPersistentSession<'a> {
    user_id: i32,
    hashed_token: &'a SecureToken,
}

impl NewPersistentSession<'_> {
    /// Inserts the session into the database.
    pub fn insert(self, conn: &PgConnection) -> Result<PersistentSession, diesel::result::Error> {
        diesel::insert_into(persistent_sessions::table)
            .values(self)
            .get_result(conn)
    }
}

/// Holds the information needed for the session cookie.
#[derive(Debug, PartialEq, Eq)]
pub struct SessionCookie {
    /// The session ID in the database.
    id: i64,
    /// The token
    token: String,
}

impl SessionCookie {
    /// Name of the cookie used for session-based authentication.
    pub const SESSION_COOKIE_NAME: &'static str = "__Host-auth";

    /// Creates a new `SessionCookie`.
    pub fn new(id: i64, token: String) -> Self {
        Self { id, token }
    }

    /// Returns the `[Cookie]`.
    pub fn build(&self, secure: bool) -> Cookie<'static> {
        Cookie::build(
            Self::SESSION_COOKIE_NAME,
            format!("{}:{}", self.id, &self.token),
        )
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .path("/")
        .finish()
    }

    pub fn session_id(&self) -> i64 {
        self.id
    }

    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Error returned when the session cookie couldn't be parsed.
#[derive(Error, Debug, PartialEq)]
pub enum ParseSessionCookieError {
    #[error("The session id wasn't in the cookie.")]
    MissingSessionId,
    #[error("The session id couldn't be parsed from the cookie.")]
    IdParseError(#[from] ParseIntError),
    #[error("The session token wasn't in the cookie.")]
    MissingToken,
}

impl FromStr for SessionCookie {
    type Err = ParseSessionCookieError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut id_and_token = s.split(':');
        let id: i64 = id_and_token
            .next()
            .ok_or(ParseSessionCookieError::MissingSessionId)?
            .parse()?;
        let token = id_and_token
            .next()
            .ok_or(ParseSessionCookieError::MissingToken)?;

        Ok(Self::new(id, token.to_string()))
    }
}
