use std::{
    fs::File,
    io::{BufRead, BufReader},
    thread,
    time::Duration,
};

use anyhow::Context;
use cargo_registry_index::{Repository, RepositoryConfig};
use diesel::prelude::*;
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};

use crate::{
    admin::dialoguer,
    db,
    schema::{crates, dependencies, versions},
};

#[derive(clap::Parser, Debug, Copy, Clone)]
#[command(
    name = "git-import",
    about = "Import missing fields from git into the database"
)]
pub struct Opts {
    /// Time in milliseconds to sleep between crate updates to reduce database load.
    #[arg(long)]
    delay: u64,
}

pub fn run(opts: Opts) -> anyhow::Result<()> {
    let conn = db::oneoff_connection().unwrap();
    println!("fetching git repo");
    let config = RepositoryConfig::from_environment();
    let repo = Repository::open(&config)?;
    repo.reset_head()?;
    println!("HEAD is at {}", repo.head_oid()?);
    let files = repo.get_files_modified_since(None)?;
    println!("found {} crates", files.len());
    if !dialoguer::confirm("continue?") {
        return Ok(());
    }

    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(ProgressStyle::with_template("{bar:60} ({pos}/{len}, ETA {eta})").unwrap());

    for file in files.iter().progress_with(pb.clone()) {
        thread::sleep(Duration::from_millis(opts.delay));
        let crate_name = file.file_name().unwrap().to_str().unwrap();
        let path = repo.index_file(crate_name);
        if !path.exists() {
            pb.suspend(|| println!("skipping {}", path.display()));
            continue;
        }
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let result = conn.transaction(|| -> anyhow::Result<()> {
            for line in reader.lines() {
                let krate: cargo_registry_index::Crate = serde_json::from_str(&line?)?;
                import_data(&conn, &krate).with_context(|| {
                    format!("Failed to update crate {}#{}", krate.name, krate.vers)
                })?
            }
            Ok(())
        });
        if let Err(err) = result {
            pb.suspend(|| println!("{err:?}"));
        }
    }
    println!("completed");

    Ok(())
}

fn import_data(conn: &PgConnection, krate: &cargo_registry_index::Crate) -> anyhow::Result<()> {
    let version_id: i32 = versions::table
        .inner_join(crates::table)
        .filter(crates::name.eq(&krate.name))
        .filter(versions::num.eq(&krate.vers))
        .select(versions::id)
        .first(conn)
        .with_context(|| {
            format!(
                "Failed to find {}#{} in the database",
                krate.name, krate.vers
            )
        })?;

    // Update the `checksum` and `links` fields.
    diesel::update(versions::table)
        .set((
            versions::checksum.eq(&krate.cksum),
            versions::links.eq(&krate.links),
        ))
        .filter(versions::id.eq(version_id))
        .filter(versions::checksum.is_null())
        .filter(versions::links.is_null())
        .execute(conn)
        .with_context(|| {
            format!(
                "Failed to update checksum/links of {}#{} (id: {version_id})",
                krate.name, krate.vers
            )
        })?;

    // Check if any of this crate's dependencies have a missing explicit_name.
    if krate.deps.iter().any(|d| d.package.is_some())
        && dependencies::table
            .filter(dependencies::version_id.eq(version_id))
            .filter(dependencies::explicit_name.is_not_null())
            .count()
            .get_result::<i64>(conn)?
            == 0
    {
        for dep in &krate.deps {
            if let Some(package) = &dep.package {
                // This is a little tricky because there can be two identical deps in the
                // database. The only difference in git is the field we're trying to
                // fill (explicit_name). Using `first` here & filtering out existing `explicit_name`
                // entries ensure that we assign one explicit_name to each dep.

                let id: i32 = dependencies::table
                    .inner_join(crates::table)
                    .filter(dependencies::explicit_name.is_null())
                    .filter(dependencies::version_id.eq(version_id))
                    .filter(dependencies::req.eq(&dep.req))
                    .filter(dependencies::features.eq(&dep.features))
                    .filter(dependencies::optional.eq(&dep.optional))
                    .filter(dependencies::default_features.eq(&dep.default_features))
                    .filter(dependencies::target.is_not_distinct_from(&dep.target))
                    .filter(dependencies::kind.eq(dep.kind.map(|k| k as i32).unwrap_or_default()))
                    .filter(crates::name.eq(package))
                    .select(dependencies::id)
                    .first(conn)
                    .with_context(|| {
                        format!(
                            "{}#{}: Failed to find matching dependency: {} {}",
                            krate.name, krate.vers, package, dep.req
                        )
                    })?;

                diesel::update(dependencies::table)
                    .set(dependencies::explicit_name.eq(&dep.name))
                    .filter(dependencies::id.eq(id))
                    .execute(conn)
                    .with_context(|| {
                        format!(
                            "Failed to update `explicit_name` of dependency {id} to {}",
                            dep.name
                        )
                    })?;
            }
        }
    }
    Ok(())
}
