// Populate a set of dummy download statistics for a specific version in the
// database.
//
// Usage:
//      cargo run --bin populate version_id1 version_id2 ...

extern crate "cargo-registry" as cargo_registry;
extern crate postgres;
extern crate time;

use std::os;
use std::time::Duration;
use std::rand::{StdRng, Rng};

fn main() {
    let conn = postgres::Connection::connect(env("DATABASE_URL").as_slice(),
                                             &postgres::SslMode::None).unwrap();
    {
        let tx = conn.transaction().unwrap();
        update(&tx).unwrap();
        tx.set_commit();
        tx.finish().unwrap();
    }
}

fn env(s: &str) -> String {
    match os::getenv(s) {
        Some(s) => s,
        None => panic!("must have `{}` defined", s),
    }
}

fn update(tx: &postgres::Transaction) -> postgres::Result<()> {
    let ids = os::args();
    let mut ids = ids.iter().skip(1).filter_map(|arg| {
        from_str::<i32>(arg.as_slice())
    });
    for id in ids {
        let now = time::now_utc().to_timespec();
        let mut rng = StdRng::new().unwrap();
        let mut dls = rng.gen_range(5000i32, 10000);

        for day in range(0, 90) {
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
