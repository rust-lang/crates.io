use std::error::Error;

use conduit_middleware;
use conduit::Request;
use conduit_cookie::RequestSession;
use diesel::prelude::*;

use db::RequestTransaction;
use schema::users;
use super::User;
use util::errors::{std_error, CargoResult, ChainError, Unauthorized};

#[derive(Debug, Clone, Copy)]
pub struct Middleware;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum AuthenticationSource {
    SessionCookie,
    ApiToken,
}

impl conduit_middleware::Middleware for Middleware {
    fn before(&self, req: &mut Request) -> Result<(), Box<Error + Send>> {
        // Check if the request has a session cookie with a `user_id` property inside
        let id = {
            req.session()
                .get("user_id")
                .and_then(|s| s.parse::<i32>().ok())
        };

        let conn = req.db_conn().map_err(std_error)?;

        if let Some(id) = id {
            // If it did, look for a user in the database with the given `user_id`
            let maybe_user = users::table.find(id).first::<User>(&*conn);
            if let Ok(user) = maybe_user {
                // Attach the `User` model from the database to the request
                req.mut_extensions().insert(user);
                req.mut_extensions()
                    .insert(AuthenticationSource::SessionCookie);
            }
        } else {
            // Otherwise, look for an `Authorization` header on the request
            // and try to find a user in the database with a matching API token
            let user = if let Some(headers) = req.headers().find("Authorization") {
                User::find_by_api_token(&conn, headers[0]).ok()
            } else {
                None
            };
            if let Some(user) = user {
                // Attach the `User` model from the database to the request
                req.mut_extensions().insert(user);
                req.mut_extensions().insert(AuthenticationSource::ApiToken);
            }
        }

        Ok(())
    }
}

pub trait RequestUser {
    fn user(&self) -> CargoResult<&User>;
    fn authentication_source(&self) -> CargoResult<AuthenticationSource>;
}

impl<'a> RequestUser for Request + 'a {
    fn user(&self) -> CargoResult<&User> {
        self.extensions()
            .find::<User>()
            .chain_error(|| Unauthorized)
    }

    fn authentication_source(&self) -> CargoResult<AuthenticationSource> {
        self.extensions()
            .find::<AuthenticationSource>()
            .cloned()
            .chain_error(|| Unauthorized)
    }
}
