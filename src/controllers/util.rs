use super::prelude::*;

use crate::middleware::current_user::TrustedUserId;
use crate::middleware::log_request;
use crate::models::{ApiToken, User};
use crate::util::errors::{
    forbidden, internal, AppError, AppResult, ChainError, InsecurelyGeneratedTokenRevoked,
};

#[derive(Debug)]
pub struct AuthenticatedUser {
    user_id: i32,
    token_id: Option<i32>,
}

impl AuthenticatedUser {
    pub fn user_id(&self) -> i32 {
        self.user_id
    }

    pub fn api_token_id(&self) -> Option<i32> {
        self.token_id
    }

    pub fn find_user(&self, conn: &PgConnection) -> AppResult<User> {
        User::find(conn, self.user_id())
            .chain_error(|| internal("user_id from cookie or token not found in database"))
    }
}

// The Origin header (https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Origin)
// is sent with CORS requests and POST requests, and indicates where the request comes from.
// We don't want to accept authenticated requests that originated from other sites, so this
// function returns an error if the Origin header doesn't match what we expect "this site" to
// be: https://crates.io in production, or http://localhost:port/ in development.
fn verify_origin(req: &dyn RequestExt) -> AppResult<()> {
    let headers = req.headers();
    let allowed_origins = req
        .app()
        .config
        .allowed_origins
        .iter()
        .map(|s| &**s)
        .collect::<Vec<_>>();

    let bad_origin = headers
        .get_all(header::ORIGIN)
        .iter()
        .find(|h| !allowed_origins.contains(&h.to_str().unwrap_or_default()));
    if let Some(bad_origin) = bad_origin {
        let error_message = format!(
            "only same-origin requests can be authenticated. got {:?}",
            bad_origin
        );
        return Err(internal(&error_message))
            .chain_error(|| Box::new(forbidden()) as Box<dyn AppError>);
    }
    Ok(())
}

impl<'a> UserAuthenticationExt for dyn RequestExt + 'a {
    /// Obtain `AuthenticatedUser` for the request or return an `Forbidden` error
    fn authenticate(&mut self) -> AppResult<AuthenticatedUser> {
        verify_origin(self)?;
        if let Some(id) = self.extensions().find::<TrustedUserId>().map(|x| x.0) {
            log_request::add_custom_metadata(self, "uid", id);
            Ok(AuthenticatedUser {
                user_id: id,
                token_id: None,
            })
        } else {
            // Otherwise, look for an `Authorization` header on the request
            let maybe_authorization: Option<String> = {
                self.headers()
                    .get(header::AUTHORIZATION)
                    .and_then(|h| h.to_str().ok())
                    .map(|h| h.to_string())
            };
            if let Some(header_value) = maybe_authorization {
                let user = {
                    let conn = self.db_conn()?;
                    ApiToken::find_by_api_token(&conn, &header_value)
                        .map(|token| AuthenticatedUser {
                            user_id: token.user_id,
                            token_id: Some(token.id),
                        })
                        .map_err(|e| {
                            if e.is::<InsecurelyGeneratedTokenRevoked>() {
                                e
                            } else {
                                e.chain(internal("invalid token")).chain(forbidden())
                            }
                        })?
                };
                log_request::add_custom_metadata(self, "uid", user.user_id);
                log_request::add_custom_metadata(self, "tokenid", user.token_id.unwrap_or(0));
                Ok(user)
            } else {
                // Unable to authenticate the user
                Err(internal("no cookie session or auth header found")).chain_error(forbidden)
            }
        }
    }
}
