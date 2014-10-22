use std::fmt::Show;

use conduit::Request;
use conduit_middleware::Middleware;
use semver;

use cargo_registry::db::RequestTransaction;
use cargo_registry::{Crate, User, Dependency};

pub struct MockUser(pub User);

impl Middleware for MockUser {
    fn before(&self, req: &mut Request) -> Result<(), Box<Show + 'static>> {
        let MockUser(ref u) = *self;
        ::mock_user(req, u.clone());
        Ok(())
    }
}

pub struct MockCrate(pub Crate);

impl Middleware for MockCrate {
    fn before(&self, req: &mut Request) -> Result<(), Box<Show + 'static>> {
        let MockCrate(ref p) = *self;
        ::mock_crate(req, p.clone());
        Ok(())
    }
}

pub struct MockDependency(pub Crate, pub Crate);

impl Middleware for MockDependency {
    fn before(&self, req: &mut Request) -> Result<(), Box<Show + 'static>> {
        let MockDependency(ref a, ref b) = *self;
        let crate_a = ::mock_crate(req, a.clone());
        let crate_b = ::mock_crate(req, b.clone());
        let va = crate_a.versions(req.tx().unwrap()).unwrap()[0].id;
        Dependency::insert(req.tx().unwrap(), va, crate_b.id,
                           &semver::VersionReq::parse(">= 0").unwrap(),
                           false, true, [], &None).unwrap();
        Ok(())
    }
}
