use std::error::Error;

use conduit::Request;
use conduit_middleware::Middleware;

use cargo_registry::{Crate, User};

pub struct MockUser(pub User);

impl Middleware for MockUser {
    fn before(&self, req: &mut Request) -> Result<(), Box<Error+Send>> {
        let MockUser(ref u) = *self;
        ::mock_user(req, u.clone());
        Ok(())
    }
}

pub struct MockCrate(pub Crate);

impl Middleware for MockCrate {
    fn before(&self, req: &mut Request) -> Result<(), Box<Error+Send>> {
        let MockCrate(ref p) = *self;
        ::mock_crate(req, p.clone());
        Ok(())
    }
}
