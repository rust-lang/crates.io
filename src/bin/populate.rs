// Populate a set of dummy download statistics for a specific version in the
// database.
//
// Usage:
//      cargo run --bin populate version_id1 version_id2 ...

#![deny(warnings)]

extern crate cargo_registry;
extern crate postgres;
extern crate time;
extern crate rand;

use std::env;
use time::Duration;
use rand::{StdRng, Rng};

use cargo_registry::env;

#[allow(dead_code)]
fn main() {
    let conn = postgres::Connection::connect(&env("DATABASE_URL")[..],
                                             postgres::TlsMode::None).unwrap();
    {
        let tx = conn.transaction().unwrap();
        update(&tx).unwrap();
        tx.set_commit();
        tx.finish().unwrap();
    }
}

fn update(tx: &postgres::transaction::Transaction) -> postgres::Result<()> {
    let ids = env::args().skip(1).filter_map(|arg| {
        arg.parse::<i32>().ok()
    });
    for id in ids {
        let now = time::now_utc().to_timespec();
        let mut rng = StdRng::new().unwrap();
        let mut dls = rng.gen_range(5000i32, 10000);

        for day in 0..90 {
            let moment = now + Duration::days(-day);
            dls += rng.gen_range(-100, 100);
            try!(tx.execute("INSERT INTO version_downloads \
                              (version_id, downloads, date) \
                              VALUES ($1, $2, $3)",
                            &[&id, &dls, &moment]));
        }
    }
    Ok(())
}
