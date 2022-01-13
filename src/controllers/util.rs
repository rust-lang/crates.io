use chrono::Utc;
use conduit_cookie::RequestSession;

use super::prelude::*;

use crate::middleware::log_request;
use crate::models::{ApiToken, User};
use crate::util::errors::{
    account_locked, forbidden, internal, AppError, AppResult, InsecurelyGeneratedTokenRevoked,
};

#[derive(Debug)]
pub struct AuthenticatedUser {
    user: User,
    token_id: Option<i32>,
}

impl AuthenticatedUser {
    pub fn user_id(&self) -> i32 {
        self.user.id
    }

    pub fn api_token_id(&self) -> Option<i32> {
        self.token_id
    }

    pub fn user(self) -> User {
        self.user
    }

    /// Disallows token authenticated users
    pub fn forbid_api_token_auth(self) -> AppResult<Self> {
        if self.token_id.is_none() {
            Ok(self)
        } else {
            Err(
                internal("API Token authentication was explicitly disallowed for this API")
                    .chain(forbidden()),
            )
        }
    }
}

/// The Origin header (https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Origin)
/// is sent with CORS requests and POST requests, and indicates where the request comes from.
/// We don't want to accept authenticated requests that originated from other sites, so this
/// function returns an error if the Origin header doesn't match what we expect "this site" to
/// be: https://crates.io in production, or http://localhost:port/ in development.
fn verify_origin(req: &dyn RequestExt) -> AppResult<()> {
    let headers = req.headers();
    let allowed_origins = &req.app().config.allowed_origins;

    let bad_origin = headers
        .get_all(header::ORIGIN)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| !allowed_origins.iter().any(|it| it == value));

    if let Some(bad_origin) = bad_origin {
        let error_message =
            format!("only same-origin requests can be authenticated. got {bad_origin:?}");
        return Err(internal(&error_message).chain(forbidden()));
    }
    Ok(())
}

fn authenticate_user(req: &dyn RequestExt) -> AppResult<AuthenticatedUser> {
    let conn = req.db_conn()?;

    let session = req.session();
    let user_id_from_session = session.get("user_id").and_then(|s| s.parse::<i32>().ok());

    if let Some(id) = user_id_from_session {
        let user = User::find(&conn, id)
            .map_err(|err| err.chain(internal("user_id from cookie not found in database")))?;

        return Ok(AuthenticatedUser {
            user,
            token_id: None,
        });
    }

    // Otherwise, look for an `Authorization` header on the request
    let maybe_authorization = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if let Some(header_value) = maybe_authorization {
        let token = ApiToken::find_by_api_token(&conn, header_value).map_err(|e| {
            if e.is::<InsecurelyGeneratedTokenRevoked>() {
                e
            } else {
                e.chain(internal("invalid token")).chain(forbidden())
            }
        })?;

        let user = User::find(&conn, token.user_id)
            .map_err(|err| err.chain(internal("user_id from token not found in database")))?;

        return Ok(AuthenticatedUser {
            user,
            token_id: Some(token.id),
        });
    }

    // Unable to authenticate the user
    return Err(internal("no cookie session or auth header found").chain(forbidden()));
}

impl<'a> UserAuthenticationExt for dyn RequestExt + 'a {
    /// Obtain `AuthenticatedUser` for the request or return an `Forbidden` error
    fn authenticate(&mut self) -> AppResult<AuthenticatedUser> {
        verify_origin(self)?;

        let authenticated_user = authenticate_user(self)?;

        if let Some(reason) = &authenticated_user.user.account_lock_reason {
            let still_locked = if let Some(until) = authenticated_user.user.account_lock_until {
                until > Utc::now().naive_utc()
            } else {
                true
            };
            if still_locked {
                return Err(account_locked(
                    reason,
                    authenticated_user.user.account_lock_until,
                ));
            }
        }

        log_request::add_custom_metadata(self, "uid", authenticated_user.user_id());
        if let Some(id) = authenticated_user.api_token_id() {
            log_request::add_custom_metadata(self, "tokenid", id);
        }

        Ok(authenticated_user)
    }
}
