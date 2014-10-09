extern crate "cargo-registry" as cargo_registry;
extern crate postgres;
extern crate time;

use std::os;
use postgres::{PostgresConnection, PostgresResult, PostgresTransaction};
use std::time::Duration;
use std::rand::{StdRng, Rng};

fn main() {
    let conn = PostgresConnection::connect(env("DATABASE_URL").as_slice(),
                                           &postgres::NoSsl).unwrap();
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
        None => fail!("must have `{}` defined", s),
    }
}

fn update(tx: &PostgresTransaction) -> PostgresResult<()> {
    for &id in [48i32, 49, 50, 51, 52, 53].iter() {
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
