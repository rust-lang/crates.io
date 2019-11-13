use cargo_registry::util::{human, CargoResult};
use cargo_registry::{db, env, tasks};
use swirl::Job;

fn main() -> CargoResult<()> {
    let conn = db::connect_now()?;
    let mut args = std::env::args().skip(1);
    match &*args.next().unwrap_or_default() {
        "update_downloads" => tasks::update_downloads().enqueue(&conn)?,
        "dump_db" => {
            let database_url = args.next().unwrap_or_else(|| env("DATABASE_URL"));
            let target_name = args
                .next()
                .unwrap_or_else(|| String::from("db-dump.tar.gz"));
            tasks::dump_db(database_url, target_name).enqueue(&conn)?
        }
        other => return Err(human(&format!("Unrecognized job type `{}`", other))),
    }
    Ok(())
}
