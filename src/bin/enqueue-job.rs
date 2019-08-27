use cargo_registry::util::{human, CargoError, CargoResult};
use cargo_registry::{db, tasks};
use std::env::args;
use swirl::Job;

fn main() -> CargoResult<()> {
    match &*args().nth(1).unwrap_or_default() {
        "update_downloads" => enqueue(tasks::update_downloads()),
        "dump_db" => enqueue(tasks::dump_db()),
        other => Err(human(&format!("Unrecognized job type `{}`", other))),
    }
}

fn enqueue<J: Job>(job: J) -> CargoResult<()> {
    let conn = db::connect_now()?;
    job.enqueue(&conn)
        .map_err(|e| CargoError::from_std_error(e))
}
