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

fn main() {
    let conn = postgres::Connection::connect(&env("DATABASE_URL")[..],
                                             &postgres::SslMode::None).unwrap();
    {
        let tx = conn.transaction().unwrap();
        update(&tx).unwrap();
        tx.set_commit();
        tx.finish().unwrap();
    }
}

fn env(s: &str) -> String {
    match env::var(s).ok() {
        Some(s) => s,
        None => panic!("must have `{}` defined", s),
    }
}

fn update(tx: &postgres::Transaction) -> postgres::Result<()> {
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
                              (version_id, downloads, counted, date, processed) \
                              VALUES ($1, $2, 0, $3, false)",
                            &[&id, &dls, &moment]));
        }
    }
    Ok(())
}
