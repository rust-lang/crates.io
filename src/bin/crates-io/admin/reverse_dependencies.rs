use anyhow::{Context, bail};
use crates_io::db;
use crates_io::schema::{crates, reverse_dependencies};
use crates_io_database::fns::rebuild_reverse_dependencies;
use diesel::prelude::*;
use diesel::sql_types::Integer;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};
use std::collections::HashSet;
use std::num::NonZeroUsize;

#[derive(clap::Parser, Debug)]
#[clap(
    name = "reverse-dependencies",
    about = "Rebuild or verify the `reverse_dependencies` table for specific crates or all of them."
)]
pub enum Command {
    /// Rebuild the reverse-dependency edges. Used for the initial backfill and
    /// as a repair tool.
    Update(UpdateArgs),
    /// Recompute the reverse-dependency edges and compare them to the stored
    /// rows, logging a warning for any divergence.
    Verify(Scope),
}

/// Options for the `update` subcommand.
#[derive(clap::Args, Debug)]
pub struct UpdateArgs {
    #[command(flatten)]
    scope: Scope,

    /// How many crates to rebuild per database call.
    #[arg(long, default_value = "1000")]
    chunk_size: NonZeroUsize,
}

/// Which crates to operate on: either every crate (`--all`) or an explicit list
/// of crate names.
#[derive(clap::Args, Debug)]
pub struct Scope {
    /// Process every crate.
    #[arg(long, conflicts_with = "crates")]
    all: bool,
    /// Names of the crates to process. Required unless `--all` is given.
    #[arg(value_name = "CRATE", required_unless_present = "all")]
    crates: Vec<String>,
}

pub async fn run(command: Command) -> anyhow::Result<()> {
    let mut conn = db::oneoff_connection()
        .await
        .context("Failed to connect to the database")?;

    let (scope, update_chunk_size) = match command {
        Command::Update(args) => (args.scope, Some(args.chunk_size)),
        Command::Verify(scope) => (scope, None),
    };

    let crate_ids = resolve_crate_ids(&scope, &mut conn).await?;

    let pb = ProgressBar::new(crate_ids.len() as u64);
    pb.set_style(ProgressStyle::with_template(
        "{bar:60} ({pos}/{len}, ETA {eta})",
    )?);

    if let Some(chunk_size) = update_chunk_size {
        for chunk in crate_ids.chunks(chunk_size.get()) {
            let result = diesel::select(rebuild_reverse_dependencies(chunk))
                .execute(&mut conn)
                .await;

            if let Err(error) = result {
                pb.suspend(|| warn!("Failed to rebuild a chunk of reverse dependencies: {error}"));
            }

            pb.inc(chunk.len() as u64);
        }
    } else {
        // Verification compares per crate, so there is nothing to batch.
        let mut diverged = 0;
        let mut failed = 0;
        for crate_id in crate_ids.into_iter().progress_with(pb.clone()) {
            match verify_reverse_dependencies(crate_id, &conn).await {
                Ok(true) => {}
                Ok(false) => diverged += 1,
                Err(error) => {
                    failed += 1;
                    pb.suspend(|| {
                        warn!("Failed to verify the reverse dependencies for crate {crate_id}: {error}")
                    });
                }
            }
        }

        // Surface divergence (and any crate we could not check) as a non-zero
        // exit, so an automated run does not mistake a diverged table for a
        // healthy one.
        if diverged > 0 || failed > 0 {
            bail!(
                "Reverse dependency verification failed: {diverged} crates diverged, \
                {failed} could not be checked"
            );
        }
    }

    Ok(())
}

/// Resolves the crate ids to operate on: every crate when `--all` is given,
/// otherwise the crates named on the command line (warning about any name that
/// does not exist).
async fn resolve_crate_ids(
    scope: &Scope,
    conn: &mut AsyncPgConnection,
) -> anyhow::Result<Vec<i32>> {
    if scope.all {
        return crates::table
            .select(crates::id)
            .load(conn)
            .await
            .context("Failed to load crates");
    }

    let found: Vec<(i32, String)> = crates::table
        .filter(crates::name.eq_any(&scope.crates))
        .select((crates::id, crates::name))
        .load(conn)
        .await
        .context("Failed to look up crates")?;

    let found_names: HashSet<&str> = found.iter().map(|(_, name)| name.as_str()).collect();
    for name in &scope.crates {
        if !found_names.contains(name.as_str()) {
            warn!("Crate `{name}` not found; skipping");
        }
    }

    Ok(found.into_iter().map(|(id, _)| id).collect())
}

/// A single reverse-dependency edge, reduced to the columns that define its
/// identity. Used by [`verify_reverse_dependencies()`] to compare the stored rows
/// against a fresh computation. Download counts are intentionally excluded since
/// they are allowed to lag.
#[derive(QueryableByName, HasQuery, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[diesel(table_name = reverse_dependencies)]
struct ReverseDependency {
    target_crate_id: i32,
    dependency_id: i32,
}

/// Computes the reverse-dependency edges that should exist for the given
/// dependent crate by invoking the `compute_reverse_dependencies()` SQL
/// function, without touching the stored `reverse_dependencies` rows.
#[tracing::instrument(skip(conn))]
async fn compute_reverse_dependencies(
    crate_id: i32,
    mut conn: &AsyncPgConnection,
) -> QueryResult<Vec<ReverseDependency>> {
    let query = "SELECT target_crate_id, dependency_id FROM compute_reverse_dependencies(ARRAY[$1]::integer[])";

    diesel::sql_query(query)
        .bind::<Integer, _>(crate_id)
        .load(&mut conn)
        .await
}

/// Loads the stored reverse-dependency edges for the given dependent crate from
/// the `reverse_dependencies` table.
#[tracing::instrument(skip(conn))]
async fn query_reverse_dependencies(
    crate_id: i32,
    mut conn: &AsyncPgConnection,
) -> QueryResult<Vec<ReverseDependency>> {
    ReverseDependency::query()
        .filter(reverse_dependencies::dependent_crate_id.eq(crate_id))
        .load(&mut conn)
        .await
}

/// Verifies that the stored reverse-dependency edges for the given dependent
/// crate match a fresh computation, logging a warning on any divergence.
///
/// Returns `true` if the stored edges are consistent.
#[tracing::instrument(skip(conn))]
async fn verify_reverse_dependencies(crate_id: i32, conn: &AsyncPgConnection) -> QueryResult<bool> {
    let mut expected = compute_reverse_dependencies(crate_id, conn).await?;
    expected.sort();

    let mut stored = query_reverse_dependencies(crate_id, conn).await?;
    stored.sort();

    let is_consistent = expected == stored;
    if is_consistent {
        debug!("Reverse dependencies for crate {crate_id} are consistent");
    } else {
        let expected_count = expected.len();
        let stored_count = stored.len();
        warn!(
            "Reverse dependencies for crate {crate_id} are outdated \
             (expected {expected_count} edges, stored {stored_count} edges)"
        );
    }

    Ok(is_consistent)
}
