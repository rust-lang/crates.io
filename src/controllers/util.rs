use super::prelude::*;

use crate::middleware::current_user::TrustedUserId;
use crate::models::{ApiToken, User};
use crate::util::errors::{internal, AppError, AppResult, ChainError, Unauthorized};

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
    /// Obtain `AuthenticatedUser` for the request or return an `Unauthorized` error
    fn authenticate(&self, conn: &PgConnection) -> AppResult<AuthenticatedUser> {
        let forwarded_host = self.headers().get("x-forwarded-host");
        let forwarded_proto = self.headers().get("x-forwarded-proto");
        let expected_origin = match (forwarded_host, forwarded_proto) {
            (Some(host), Some(proto)) => format!(
                "{}://{}",
                proto.to_str().unwrap_or_default(),
                host.to_str().unwrap_or_default()
            ),
            _ => "".to_string(),
        };

        let bad_origin = self
            .headers()
            .get_all(header::ORIGIN)
            .iter()
            .find(|h| h.to_str().unwrap_or_default() != expected_origin);
        if let Some(bad_origin) = bad_origin {
            let error_message = format!(
                "only same-origin requests can be authenticated. expected {}, got {:?}",
                expected_origin, bad_origin
            );
            return Err(internal(&error_message))
                .chain_error(|| Box::new(Unauthorized) as Box<dyn AppError>);
        }
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
                    .chain_error(|| internal("invalid token"))
                    .chain_error(|| Box::new(Unauthorized) as Box<dyn AppError>)
            } else {
                // Unable to authenticate the user
                Err(internal("no cookie session or auth header found"))
                    .chain_error(|| Box::new(Unauthorized) as Box<dyn AppError>)
            }
        }
    }
}
