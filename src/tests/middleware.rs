use std::error::Error;

use conduit::Request;
use conduit_middleware::Middleware;
use semver;

use cargo_registry::db::RequestTransaction;
use cargo_registry::{Crate, User, Dependency};
use cargo_registry::dependency::Kind;

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

pub struct MockDependency(pub Crate, pub &'static str, pub Crate);

impl Middleware for MockDependency {
    fn before(&self, req: &mut Request) -> Result<(), Box<Error+Send>> {
        let MockDependency(ref a, version, ref b) = *self;
        let vers = semver::Version::parse(version).unwrap();
        let (_crate_a, va) = ::mock_crate_vers(req, a.clone(), &vers);

        // don't panic on duplicate uploads
        let (crate_b, _) = ::mock_crate_vers(req, b.clone(),
                                             &semver::Version::parse("1.0.0").unwrap());

        Dependency::insert(req.tx().unwrap(), va.unwrap().id, crate_b.id,
                           &semver::VersionReq::parse(">= 0").unwrap(),
                           Kind::Normal,
                           false, true, &[], &None).unwrap();
        Ok(())
    }
}
