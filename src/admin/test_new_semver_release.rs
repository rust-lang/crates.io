use crate::{
    db,
    schema::{dependencies, versions},
};

use clap::Clap;
use diesel::prelude::*;

#[derive(Clap, Copy, Clone, Debug)]
#[clap(
    name = "test-new-semver-release",
    about = "Scan the database for all versions and version requirements to \
        ensure they can be parsed by the latest `semver` crate."
)]
pub struct Opts {}

pub fn run(_opts: Opts) {
    use self::dependencies::dsl::*;
    use self::versions::dsl::*;

    let url = db::connection_url(&dotenv::var("READ_ONLY_REPLICA_URL").unwrap());
    let conn = PgConnection::establish(&url).unwrap();
    conn.transaction::<_, diesel::result::Error, _>(|| {
        let vers = versions.select(num).load(&conn)?;
        test_versions(&vers);

        let deps = dependencies.select(req).load(&conn)?;
        test_dependency_predicates(&deps);

        Ok(())
    })
    .unwrap();

    println!("Test finished.");
}

fn test_versions(versions: &[String]) {
    for version in versions {
        if let Err(e) = semver::Version::parse(version) {
            println!("Could not parse `{}` as a semver::Version: {}", version, e);
        }
    }

    for version in versions {
        if let Err(e) = semver::Version::parse(version) {
            println!("Could not parse `{}` as a semver::Version: {}", version, e);
        }

        if let Err(e) = semver_next::Version::parse(version) {
            println!(
                "Could not parse `{}` as a semver_next::Version: {}",
                version, e
            );
        }
    }
}

fn test_dependency_predicates(versions: &[String]) {
    for version in versions {
        if let Err(e) = semver::VersionReq::parse(version) {
            println!("Could not parse `{}` as a semver::Version: {}", version, e);
        }

        if let Err(e) = semver_next::VersionReq::parse(version) {
            println!(
                "Could not parse `{}` as a semver_next::Version: {}",
                version, e
            );
        }
    }
}
