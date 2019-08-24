use cargo_registry::util::{human, CargoError, CargoResult};
use cargo_registry::{db, env, tasks};
use diesel::PgConnection;

fn main() -> CargoResult<()> {
    let conn = db::connect_now()?;
    let mut args = std::env::args().skip(1);
    match &*args.next().unwrap_or_default() {
        "update_downloads" => tasks::update_downloads().enqueue(&conn),
        "dump_db" => {
            let database_url = args.next().unwrap_or_else(|| env("DATABASE_URL"));
            let target_name = args
                .next()
                .unwrap_or_else(|| String::from("db-dump.tar.gz"));
            tasks::dump_db(database_url, target_name).enqueue(&conn)
        }
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
