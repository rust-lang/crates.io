use std::collections::HashMap;
use std::fmt::Show;

use conduit::Request;
use conduit_middleware::Middleware;
use semver;

use cargo_registry::db::RequestTransaction;
use cargo_registry::{Crate, User, Version, Dependency};

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
        let user = req.extensions().find::<User>().unwrap();
        let crate_a = Crate::find_or_insert(req.tx().unwrap(),
                                            a.name.as_slice(),
                                            user.id, &None, &None, &None).unwrap();
        let crate_b = Crate::find_or_insert(req.tx().unwrap(),
                                            b.name.as_slice(),
                                            user.id, &None, &None, &None).unwrap();
        let va = Version::insert(req.tx().unwrap(), crate_a.id,
                                 &semver::Version::parse("1.0.0").unwrap(),
                                 &HashMap::new()).unwrap();
        Version::insert(req.tx().unwrap(), crate_b.id,
                        &semver::Version::parse("1.0.0").unwrap(),
                        &HashMap::new()).unwrap();
        Dependency::insert(req.tx().unwrap(), va.id, crate_b.id,
                           &semver::VersionReq::parse(">= 0").unwrap(),
                           false, true, []).unwrap();
        Ok(())
    }
}
