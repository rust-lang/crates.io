use std::fmt::Show;

use conduit::Request;
use conduit_middleware::Middleware;

use cargo_registry::user::User;

pub struct MockUser(pub User);

impl Middleware for MockUser {
    fn before(&self, req: &mut Request) -> Result<(), Box<Show + 'static>> {
        let MockUser(ref u) = *self;
        req.mut_extensions().insert(u.clone());
        Ok(())
    }
}
