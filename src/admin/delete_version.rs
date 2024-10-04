use crate::models::update_default_version;
use crate::schema::crates;
use crate::storage::Storage;
use crate::tasks::spawn_blocking;
use crate::worker::jobs;
use crate::{admin::dialoguer, db, schema::versions};
use anyhow::Context;
use diesel::prelude::*;

#[derive(clap::Parser, Debug)]
#[command(
    name = "delete-version",
    about = "Purge all references to a crate's version from the database.",
    after_help = "Please be super sure you want to do this before running this!"
)]
pub struct Opts {
    /// Name of the crate
    crate_name: String,

    /// Version numbers that should be deleted
    #[arg(value_name = "VERSION", required = true)]
    versions: Vec<String>,

    /// Don't ask for confirmation: yes, we are sure. Best for scripting.
    #[arg(short, long)]
    yes: bool,
}

pub async fn run(opts: Opts) -> anyhow::Result<()> {
    spawn_blocking(move || {
        let crate_name = &opts.crate_name;

        let conn = &mut db::oneoff_connection().context("Failed to establish database connection")?;

        let store = Storage::from_environment();

        let crate_id: i32 = crates::table
            .select(crates::id)
            .filter(crates::name.eq(crate_name))
            .first(conn)
            .context("Failed to look up crate id from the database")?;

        println!("Deleting the following versions of the `{crate_name}` crate:");
        println!();
        for version in &opts.versions {
            println!(" - {version}");
        }
        println!();

        if !opts.yes && !dialoguer::confirm("Do you want to permanently delete these versions?") {
            return Ok(());
        }

        conn.transaction(|conn| {
            info!(%crate_name, %crate_id, versions = ?opts.versions, "Deleting versions from the database");
            let result = diesel::delete(
                versions::table
                    .filter(versions::crate_id.eq(crate_id))
                    .filter(versions::num.eq_any(&opts.versions)),
            )
            .execute(conn);

            match result {
                Ok(num_deleted) if num_deleted == opts.versions.len() => {}
                Ok(num_deleted) => {
                    warn!(
                        %crate_name,
                        "Deleted only {num_deleted} of {num_expected} versions from the database",
                        num_expected = opts.versions.len()
                    );
                }
                Err(error) => {
                    warn!(%crate_name, ?error, "Failed to delete versions from the database")
                }
            }

            info!(%crate_name, %crate_id, "Updating default version in the database");
            if let Err(error) = update_default_version(crate_id, conn) {
                warn!(%crate_name, %crate_id, ?error, "Failed to update default version");
            }

            Ok::<_, anyhow::Error>(())
        })?;

        info!(%crate_name, "Enqueuing index sync jobs");
        if let Err(error) = jobs::enqueue_sync_to_index(crate_name, conn) {
            warn!(%crate_name, ?error, "Failed to enqueue index sync jobs");
        }

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("Failed to initialize tokio runtime")?;

        for version in &opts.versions {
            debug!(%crate_name, %version, "Deleting crate file from S3");
            if let Err(error) = rt.block_on(store.delete_crate_file(crate_name, version)) {
                warn!(%crate_name, %version, ?error, "Failed to delete crate file from S3");
            }

            debug!(%crate_name, %version, "Deleting readme file from S3");
            match rt.block_on(store.delete_readme(crate_name, version)) {
                Err(object_store::Error::NotFound { .. }) => {}
                Err(error) => {
                    warn!(%crate_name, %version, ?error, "Failed to delete readme file from S3")
                }
                Ok(_) => {}
            }
        }

        Ok(())
    }).await
}
