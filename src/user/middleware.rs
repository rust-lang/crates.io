use std::error::Error;

use conduit_middleware;
use conduit::{Request, Response};
use conduit_cookie::RequestSession;

use Model;
use db::RequestTransaction;
use super::User;
use util::errors::{CargoResult, Unauthorized, ChainError, std_error};

pub struct Middleware;

impl conduit_middleware::Middleware for Middleware {
    fn before(&self, req: &mut Request) -> Result<(), Box<Error+Send>> {
        // Check if the request has a session cookie with a `user_id` property inside
        let id = { req.session().get("user_id").and_then(|s| s.parse().ok()) };

        let user = match id {

            // `user_id` was found on the session
            Some(id) => {

                // Look for a user in the database with the given `user_id`
                match User::find(try!(req.tx().map_err(std_error)), id) {
                    Ok(user) => user,
                    Err(..) => return Ok(()),
                }
            }

            // `user_id` was *not* found on the session
            None => {

                // Look for an `Authorization` header on the request
                match req.headers().find("Authorization") {
                    Some(headers) => {

                        // Look for a user in the database with a matching API token
                        let tx = try!(req.tx().map_err(std_error));
                        match User::find_by_api_token(tx, &headers[0]) {
                            Ok(user) => user,
                            Err(..) => return Ok(())
                        }
                    }
                    None => return Ok(())
                }
            }
        };

        // Attach the `User` model from the database to the request
        req.mut_extensions().insert(user);
        Ok(())
    }

    fn after(&self, req: &mut Request, res: Result<Response, Box<Error+Send>>)
             -> Result<Response, Box<Error+Send>> {
        req.mut_extensions().remove::<User>();
        res
    }
}

pub trait RequestUser {
    fn user(&self) -> CargoResult<&User>;
}

impl<'a> RequestUser for Request + 'a {
    fn user(&self) -> CargoResult<&User> {
        self.extensions().find::<User>().chain_error(|| Unauthorized)
    }
}
