use crate::db;
use crate::schema::{crates, versions};
use anyhow::anyhow;
use cargo_registry_index::Repository;
use cargo_registry_tarball::process_tarball;
use chrono::{Days, NaiveDate};
use crossbeam_channel::unbounded;
use diesel::connection::DefaultLoadingMode;
use diesel::prelude::*;
use diesel::sql_query;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Read the `rust-version` field from the `Cargo.toml` file and generate
/// a SQL script to backfill the database.
///
/// ## Instructions
///
/// - Run `./script/import-database-dump.sh` to import the latest database
///   dump into your local database
///
/// - Install <https://crates.io/crates/get-all-crates>
///
/// - Clone <https://github.com/rust-lang/crates.io-index> to your local disk
///
/// - Run `get-all-crates --index ../crates.io-index/ --out ./tmp/all-crates` to
///   download all crate files to your disk
///
/// - Run `cargo run --bin crates-admin backfill rust-version --crates tmp/all-crates/`
///   to generate the backfilling SQL script
///
/// Note that this command will create a temporary `__rust_version_cache` table
/// in your local database to allow for pausing and restarting the processing.
/// Feel free to delete the table again once the SQL script has been created.
///
#[derive(clap::Parser, Debug, Clone)]
pub struct RustVersionOptions {
    /// Path to the `get-all-crates` output
    #[arg(long = "crates", value_name = "PATH")]
    crates_path: PathBuf,
    /// Output path for the SQL script
    #[arg(
        long = "out",
        value_name = "PATH",
        default_value = "rust-version-backfill.sql"
    )]
    output_path: PathBuf,
    /// Only include versions published after this date
    #[arg(long, value_name = "YYYY-MM-DD")]
    from: Option<NaiveDate>,
    /// Only include versions published before this date
    #[arg(long, value_name = "YYYY-MM-DD")]
    to: Option<NaiveDate>,
    /// Limit number of concurrent processing threads
    #[arg(short = 'j', value_name = "INT", default_value = "20")]
    max_concurrency: NonZeroU32,
}

table! {
    __rust_version_cache (name, version) {
        name -> Varchar,
        version -> Varchar,
        rust_version -> Nullable<Varchar>,
    }
}

use __rust_version_cache as rust_version_cache;

pub fn run(options: &RustVersionOptions) -> anyhow::Result<()> {
    if !options.crates_path.is_dir() {
        return Err(anyhow!("`{}` not found", options.crates_path.display()));
    }

    info!("Connecting to database…");
    let mut conn = db::oneoff_connection()?;

    info!("Creating temporary __rust_version_cache table…");
    sql_query(
        "CREATE TABLE IF NOT EXISTS __rust_version_cache
            (
                name         VARCHAR NOT NULL,
                version      VARCHAR NOT NULL,
                rust_version VARCHAR,
                CONSTRAINT __rust_version_cache_pk
                    PRIMARY KEY (name, version)
            )",
    )
    .execute(&mut conn)?;

    info!("Loading cached values from the database…");
    let cache = read_cache_map(&mut conn)?;
    let cache = Arc::new(cache);
    info!("Found {} cached values", cache.len());

    let (versions_sender, versions_receiver) = unbounded::<(String, String)>();
    let (result_sender, result_receiver) = unbounded();

    let max_concurrency = options.max_concurrency.get();
    info!("Spinning up {max_concurrency} worker threads…");
    let worker_threads = (1..=max_concurrency)
        .map(|i| {
            let versions_receiver = versions_receiver.clone();
            let result_sender = result_sender.clone();
            let cache = cache.clone();
            let options = options.clone();

            std::thread::spawn(move || {
                // Receive processing requests from the main thread
                while let Ok((name, version)) = versions_receiver.recv() {
                    // Only process versions that we haven't cached yet
                    let cache_key = cache_key(&name, &version);
                    if !cache.contains_key(&cache_key) {
                        match process_version(&name, &version, &options) {
                            Ok(rust_version) => {
                                info!(%name, %version, ?rust_version);

                                // Send processing result back to the main thread
                                result_sender.send((name, version, rust_version)).unwrap()
                            }
                            Err(error) => {
                                warn!(%name, %version, %error, "Failed to process version")
                            }
                        }
                    }
                }

                info!("Worker thread {i} finished");
            })
        })
        .collect::<Vec<_>>();

    drop(result_sender);

    info!("Loading relevant versions from the database…");
    let mut versions_query = versions::table
        .inner_join(crates::table)
        .select((crates::name, versions::num))
        .filter(versions::rust_version.is_null())
        .order_by((crates::name, versions::created_at))
        .into_boxed();

    if let Some(from) = options.from {
        let from = from.and_hms_opt(0, 0, 0).unwrap();
        versions_query = versions_query.filter(versions::created_at.ge(from));
    }

    if let Some(to) = options.to {
        let to = to.and_hms_opt(0, 0, 0).unwrap();
        let to = to.checked_add_days(Days::new(1)).unwrap();
        versions_query = versions_query.filter(versions::created_at.le(to));
    }

    for res in versions_query.load_iter::<(String, String), DefaultLoadingMode>(&mut conn)? {
        let (name, version) = res?;

        // Send processing request to the worker threads
        versions_sender.send((name, version))?;
    }

    drop(versions_sender);

    info!("Saving cached `rust-version` value to the database…");
    // Receive processing results from the worker threads and save the results
    // to the database.
    while let Ok((name, version, rust_version)) = result_receiver.recv() {
        diesel::insert_into(rust_version_cache::table)
            .values((
                rust_version_cache::name.eq(name),
                rust_version_cache::version.eq(version),
                rust_version_cache::rust_version.eq(rust_version),
            ))
            .execute(&mut conn)?;
    }

    for thread in worker_threads {
        thread.join().unwrap();
    }

    info!("Loading `rust-version` values from the database…");
    let cache = read_cache(&mut conn)?;

    info!("Writing SQL script…");
    let mut output_file = File::create(&options.output_path)?;
    for (name, version, rust_version) in cache.iter() {
        if let Some(rust_version) = rust_version {
            writeln!(
                &mut output_file,
                "UPDATE versions \
                SET rust_version = '{rust_version}' \
                FROM crates \
                WHERE versions.crate_id = crates.id \
                AND crates.name = '{name}' \
                AND num = '{version}';"
            )?;
        }
    }
    writeln!(&mut output_file)?;

    let crates_to_update = cache
        .into_iter()
        .filter(|(_name, _version, rust_version)| rust_version.is_some())
        .map(|(name, _version, _rust_version)| name)
        .collect::<HashSet<_>>();

    let mut crates_to_update = crates_to_update.into_iter().collect::<Vec<_>>();
    crates_to_update.sort();

    for name in crates_to_update {
        writeln!(
            &mut output_file,
            "INSERT INTO background_jobs (job_type, data, priority) \
            VALUES ('sync_to_git_index', '{{\"krate\":\"{name}\"}}'::json, -50), \
                   ('sync_to_sparse_index', '{{\"krate\":\"{name}\"}}'::json, -50);"
        )?;
    }
    writeln!(&mut output_file)?;

    Ok(())
}

fn read_cache(conn: &mut PgConnection) -> anyhow::Result<Vec<(String, String, Option<String>)>> {
    let cache = rust_version_cache::table
        .select((
            rust_version_cache::name,
            rust_version_cache::version,
            rust_version_cache::rust_version,
        ))
        .order_by((rust_version_cache::name, rust_version_cache::version))
        .load(conn)?;

    Ok(cache)
}

fn read_cache_map(conn: &mut PgConnection) -> anyhow::Result<HashMap<String, Option<String>>> {
    let cache = read_cache(conn)?;

    let cache = cache
        .into_iter()
        .map(|(name, version, rust_version)| (cache_key(&name, &version), rust_version))
        .collect();

    Ok(cache)
}

fn cache_key(name: &str, version: &str) -> String {
    format!("{name}@{version}")
}

fn process_version(
    name: &str,
    version: &str,
    options: &RustVersionOptions,
) -> anyhow::Result<Option<String>> {
    let crate_location = crate_location(&options.crates_path, name, version);

    debug!(%name, %version, crate_location = %crate_location.display(), "Reading tarball…");
    let tarball = std::fs::read(crate_location)?;

    debug!(%name, %version, "Processing tarball…");
    let pkg_name = format!("{name}-{version}");
    let info = process_tarball(&pkg_name, &tarball, u64::MAX)?;

    let rust_version = info
        .manifest
        .and_then(|m| m.package.rust_version.map(|rv| rv.to_string()));

    Ok(rust_version)
}

fn crate_location(basedir: &Path, name: &str, version: &str) -> PathBuf {
    let index_file_path = Repository::relative_index_file(name);

    let file_name = format!("{}-{}.crate", name.to_lowercase(), version.to_lowercase());
    basedir.join(index_file_path).join(file_name)
}
