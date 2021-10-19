use anyhow::Error;

static CATEGORIES_TOML: &str = include_str!("../boot/categories.toml");
diesel_migrations::embed_migrations!("./migrations");

#[derive(clap::Parser, Debug, Copy, Clone)]
#[clap(
    name = "migrate",
    about = "Verify config, migrate the database, and other release tasks."
)]
pub struct Opts;

pub fn run(_opts: Opts) -> Result<(), Error> {
    let db_config = crate::config::DatabasePools::full_from_environment();

    // TODO: Refactor logic so that we can also check things from App::new() here.
    // If the app will panic due to bad configuration, it is better to error in the release phase
    // to avoid launching dynos that will fail.

    if db_config.are_all_read_only() {
        // TODO: Check `any_pending_migrations()` with a read-only connection and error if true.
        // It looks like this requires changes upstream to make this pub in `migration_macros`.

        println!("==> Skipping migrations and category sync (read-only mode)");

        // The service is undergoing maintenance or mitigating an outage.
        // Exit with success to ensure configuration changes can be made.
        // Heroku will not launch new dynos if the release phase fails.
        return Ok(());
    }

    println!("==> migrating the database");
    // The primary is online, access directly via `DATABASE_URL`.
    let conn = crate::db::connect_now()?;
    embedded_migrations::run_with_output(&conn, &mut std::io::stdout())?;

    println!("==> synchronizing crate categories");
    crate::boot::categories::sync(CATEGORIES_TOML).unwrap();

    Ok(())
}
