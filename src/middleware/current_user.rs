use super::prelude::*;

use conduit_cookie::RequestSession;
use diesel::prelude::*;

use crate::db::RequestTransaction;
use crate::util::errors::{AppResult, ChainError, Unauthorized};

use crate::models::ApiToken;
use crate::models::User;
use crate::schema::users;

#[derive(Debug, Clone, Copy)]
pub struct CurrentUser;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum AuthenticationSource {
    SessionCookie,
    ApiToken { api_token_id: i32 },
}

impl Middleware for CurrentUser {
    fn before(&self, req: &mut dyn Request) -> Result<()> {
        // Check if the request has a session cookie with a `user_id` property inside
        let id = {
            req.session()
                .get("user_id")
                .and_then(|s| s.parse::<i32>().ok())
        };

        let conn = req.db_conn().map_err(|e| Box::new(e) as BoxError)?;

        if let Some(id) = id {
            // If it did, look for a user in the database with the given `user_id`
            let maybe_user = users::table.find(id).first::<User>(&*conn);
            drop(conn);
            if let Ok(user) = maybe_user {
                // Attach the `User` model from the database to the request
                req.mut_extensions().insert(user);
                req.mut_extensions()
                    .insert(AuthenticationSource::SessionCookie);
            }
        } else {
            // Otherwise, look for an `Authorization` header on the request
            // and try to find a user in the database with a matching API token
            let user_auth = if let Some(headers) = req.headers().find("Authorization") {
                ApiToken::find_by_api_token(&conn, headers[0])
                    .and_then(|api_token| {
                        User::find(&conn, api_token.user_id).map(|user| {
                            (
                                AuthenticationSource::ApiToken {
                                    api_token_id: api_token.id,
                                },
                                user,
                            )
                        })
                    })
                    .optional()
                    .map_err(|e| Box::new(e) as BoxError)?
            } else {
                None
            };

            drop(conn);

            if let Some((api_token, user)) = user_auth {
                // Attach the `User` model from the database and the API token to the request
                req.mut_extensions().insert(user);
                req.mut_extensions().insert(api_token);
            }
        }

        Ok(())
    }
}

pub trait RequestUser {
    fn user(&self) -> AppResult<&User>;
    fn authentication_source(&self) -> AppResult<AuthenticationSource>;
}

impl<'a> RequestUser for dyn Request + 'a {
    fn user(&self) -> AppResult<&User> {
        self.extensions()
            .find::<User>()
            .chain_error(|| Unauthorized)
    }

    fn authentication_source(&self) -> AppResult<AuthenticationSource> {
        self.extensions()
            .find::<AuthenticationSource>()
            .cloned()
            .chain_error(|| Unauthorized)
    }
}

impl AuthenticationSource {
    pub fn api_token_id(self) -> Option<i32> {
        match self {
            AuthenticationSource::SessionCookie => None,
            AuthenticationSource::ApiToken { api_token_id } => Some(api_token_id),
        }
    }
}
