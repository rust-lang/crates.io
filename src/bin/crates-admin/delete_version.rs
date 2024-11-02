use crate::dialoguer;
use anyhow::Context;
use crates_io::models::update_default_version;
use crates_io::schema::crates;
use crates_io::storage::Storage;
use crates_io::tasks::spawn_blocking;
use crates_io::worker::jobs;
use crates_io::{db, schema::versions};
use crates_io_worker::BackgroundJob;
use diesel::{Connection, ExpressionMethods, QueryDsl};
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;

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
    let mut conn = db::oneoff_connection()
        .await
        .context("Failed to establish database connection")?;

    let store = Storage::from_environment();

    let crate_id: i32 = {
        use diesel_async::RunQueryDsl;

        crates::table
            .select(crates::id)
            .filter(crates::name.eq(&opts.crate_name))
            .first(&mut conn)
            .await
            .context("Failed to look up crate id from the database")
    }?;

    {
        let crate_name = &opts.crate_name;

        println!("Deleting the following versions of the `{crate_name}` crate:");
        println!();
        for version in &opts.versions {
            println!(" - {version}");
        }
        println!();

        if !opts.yes
            && !dialoguer::async_confirm("Do you want to permanently delete these versions?")
                .await?
        {
            return Ok(());
        }
    }

    let opts = spawn_blocking::<_, _, anyhow::Error>(move || {
        use diesel::RunQueryDsl;

        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        let crate_name = &opts.crate_name;

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
        if let Err(error) = jobs::SyncToGitIndex::new(crate_name).enqueue(conn) {
            warn!(%crate_name, ?error, "Failed to enqueue SyncToGitIndex job");
        }
        if let Err(error) = jobs::SyncToSparseIndex::new(crate_name).enqueue(conn) {
            warn!(%crate_name, ?error, "Failed to enqueue SyncToSparseIndex job");
        }

        Ok(opts)
    }).await?;

    let crate_name = &opts.crate_name;

    for version in &opts.versions {
        debug!(%crate_name, %version, "Deleting crate file from S3");
        if let Err(error) = store.delete_crate_file(crate_name, version).await {
            warn!(%crate_name, %version, ?error, "Failed to delete crate file from S3");
        }

        debug!(%crate_name, %version, "Deleting readme file from S3");
        match store.delete_readme(crate_name, version).await {
            Err(object_store::Error::NotFound { .. }) => {}
            Err(error) => {
                warn!(%crate_name, %version, ?error, "Failed to delete readme file from S3")
            }
            Ok(_) => {}
        }
    }

    Ok(())
}
