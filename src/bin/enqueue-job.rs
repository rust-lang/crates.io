use cargo_registry::util::{human, CargoError, CargoResult};
use cargo_registry::{db, tasks};
use diesel::PgConnection;
use std::env::args;

fn main() -> CargoResult<()> {
    let conn = db::connect_now()?;
    match &*args().nth(1).unwrap_or_default() {
        "update_downloads" => tasks::update_downloads().enqueue(&conn),
        "dump_db" => tasks::dump_db().enqueue(&conn),
        other => Err(human(&format!("Unrecognized job type `{}`", other))),
    }
}

/// Helper to map the `PerformError` returned by `swirl::Job::enqueue()` to a
/// `CargoError`. Can be removed once `map_err()` isn't needed any more.
trait Enqueue: swirl::Job {
    fn enqueue(self, conn: &PgConnection) -> CargoResult<()> {
        <Self as swirl::Job>::enqueue(self, conn).map_err(|e| CargoError::from_std_error(e))
    }
}

impl<J: swirl::Job> Enqueue for J {}
