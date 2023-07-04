use crate::background_jobs::Job;
use crate::{admin::dialoguer, db, env, schema::crates};
use anyhow::Context;
use diesel::prelude::*;
use futures_util::{StreamExt, TryStreamExt};
use object_store::aws::{AmazonS3, AmazonS3Builder};
use object_store::ObjectStore;
use std::collections::HashMap;

#[derive(clap::Parser, Debug)]
#[command(
    name = "delete-crate",
    about = "Purge all references to a crate from the database.",
    after_help = "Please be super sure you want to do this before running this!"
)]
pub struct Opts {
    /// Names of the crates
    #[arg(value_name = "NAME", required = true)]
    crate_names: Vec<String>,

    /// Don't ask for confirmation: yes, we are sure. Best for scripting.
    #[arg(short, long)]
    yes: bool,
}

pub fn run(opts: Opts) {
    let conn = &mut db::oneoff_connection()
        .context("Failed to establish database connection")
        .unwrap();

    let region = dotenvy::var("S3_REGION").unwrap_or("us-west-1".to_string());
    let bucket = env("S3_BUCKET");
    let access_key = env("AWS_ACCESS_KEY");
    let secret_key = env("AWS_SECRET_KEY");

    let s3 = AmazonS3Builder::new()
        .with_region(region)
        .with_bucket_name(bucket)
        .with_access_key_id(access_key)
        .with_secret_access_key(secret_key)
        .build()
        .context("Failed to initialize S3 code")
        .unwrap();

    let mut crate_names = opts.crate_names;
    crate_names.sort();

    let existing_crates = crates::table
        .select((crates::name, crates::id))
        .filter(crates::name.eq_any(&crate_names))
        .load(conn)
        .context("Failed to look up crate name from the database")
        .unwrap();

    let existing_crates: HashMap<String, i32> = existing_crates.into_iter().collect();

    println!("Deleting the following crates:");
    println!();
    for name in &crate_names {
        match existing_crates.get(name) {
            Some(id) => println!(" - {name} (id={id})"),
            None => println!(" - {name} (⚠️ crate not found)"),
        }
    }
    println!();

    if !opts.yes && !dialoguer::confirm("Do you want to permanently delete these crates?") {
        return;
    }

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed to initialize tokio runtime")
        .unwrap();

    for name in &crate_names {
        if let Some(id) = existing_crates.get(name) {
            info!(%name, "Deleting crate from the database");
            if let Err(error) = diesel::delete(crates::table.find(id)).execute(conn) {
                warn!(%name, %id, ?error, "Failed to delete crate from the database");
            }
        } else {
            info!(%name, "Skipping missing crate");
        };

        info!(%name, "Enqueuing index sync jobs");
        if let Err(error) = Job::enqueue_sync_to_index(name, conn) {
            warn!(%name, ?error, "Failed to enqueue index sync jobs");
        }

        info!(%name, "Deleting crate files from S3");
        let prefix = format!("crates/{name}");
        let prefix = object_store::path::Path::from(prefix);
        if let Err(error) = rt.block_on(delete_from_s3(&s3, &prefix)) {
            warn!(%name, ?error, "Failed to delete crate files from S3");
        }

        info!(%name, "Deleting readme files from S3");
        let prefix = format!("readmes/{name}");
        let prefix = object_store::path::Path::from(prefix);
        if let Err(error) = rt.block_on(delete_from_s3(&s3, &prefix)) {
            warn!(%name, ?error, "Failed to delete readme files from S3");
        }
    }
}

async fn delete_from_s3(s3: &AmazonS3, prefix: &object_store::path::Path) -> anyhow::Result<()> {
    let objects = s3.list(Some(prefix)).await?;
    let locations = objects.map(|meta| meta.map(|m| m.location)).boxed();

    s3.delete_stream(locations)
        .try_collect::<Vec<object_store::path::Path>>()
        .await?;

    Ok(())
}
