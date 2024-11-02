use anyhow::{anyhow, Context, Error};
use crates_io::tasks::spawn_blocking;
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use diesel_async::AsyncPgConnection;
use diesel_migrations::{
    embed_migrations, EmbeddedMigrations, HarnessWithOutput, MigrationHarness,
};

static CATEGORIES_TOML: &str = include_str!("../../boot/categories.toml");

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

#[derive(clap::Parser, Debug, Copy, Clone)]
#[command(
    name = "migrate",
    about = "Verify config, migrate the database, and other release tasks."
)]
pub struct Opts;

pub async fn run(_opts: Opts) -> Result<(), Error> {
    let config = crates_io::config::DatabasePools::full_from_environment(
        &crates_io::config::Base::from_environment()?,
    )?;

    // TODO: Refactor logic so that we can also check things from App::new() here.
    // If the app will panic due to bad configuration, it is better to error in the release phase
    // to avoid launching dynos that will fail.

    if config.are_all_read_only() {
        // TODO: Check `any_pending_migrations()` with a read-only connection and error if true.
        // It looks like this requires changes upstream to make this pub in `migration_macros`.

        warn!("Skipping migrations and category sync (read-only mode)");

        // The service is undergoing maintenance or mitigating an outage.
        // Exit with success to ensure configuration changes can be made.
        // Heroku will not launch new dynos if the release phase fails.
        return Ok(());
    }

    // The primary is online, access directly via `DATABASE_URL`.
    let conn = crates_io::db::oneoff_connection()
        .await
        .context("Failed to connect to the database")?;

    let mut conn = AsyncConnectionWrapper::<AsyncPgConnection>::from(conn);

    spawn_blocking(move || {
        info!("Migrating the database");
        let mut stdout = std::io::stdout();
        let mut harness = HarnessWithOutput::new(&mut conn, &mut stdout);
        harness
            .run_pending_migrations(MIGRATIONS)
            .map_err(|err| anyhow!("Failed to run migrations: {err}"))?;

        info!("Synchronizing crate categories");
        crates_io::boot::categories::sync_with_connection(CATEGORIES_TOML, &mut conn)?;

        Ok(())
    })
    .await
}
