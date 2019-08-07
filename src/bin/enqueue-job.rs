use cargo_registry::util::{CargoError, CargoResult};
use cargo_registry::{db, tasks};
use std::env::args;
use swirl::Job;

fn main() -> CargoResult<()> {
    let conn = db::connect_now()?;

    match &*args().nth(1).unwrap_or_default() {
        "update_downloads" => tasks::update_downloads()
            .enqueue(&conn)
            .map_err(|e| CargoError::from_std_error(e))?,
        other => panic!("Unrecognized job type `{}`", other),
    };

    Ok(())
}
