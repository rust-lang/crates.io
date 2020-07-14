use super::prelude::*;

use crate::middleware::current_user::TrustedUserId;
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

impl<'a> UserAuthenticationExt for dyn RequestExt + 'a {
    /// Obtain `AuthenticatedUser` for the request or return an `Forbidden` error
    fn authenticate(&self, conn: &PgConnection) -> AppResult<AuthenticatedUser> {
        if let Some(id) = self.extensions().find::<TrustedUserId>() {
            // A trusted user_id was provided by a signed cookie (or a test `MockCookieUser`)
            Ok(AuthenticatedUser {
                user_id: id.0,
                token_id: None,
            })
        } else {
            // Otherwise, look for an `Authorization` header on the request
            if let Some(headers) = self.headers().get(header::AUTHORIZATION) {
                ApiToken::find_by_api_token(conn, headers.to_str().unwrap_or_default())
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
                    })
            } else {
                // Unable to authenticate the user
                Err(internal("no cookie session or auth header found")).chain_error(forbidden)
            }
        }
    }
}
