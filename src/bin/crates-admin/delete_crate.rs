use crate::dialoguer;
use anyhow::Context;
use colored::Colorize;
use crates_io::schema::crate_downloads;
use crates_io::worker::jobs;
use crates_io::{db, schema::crates};
use crates_io_worker::BackgroundJob;
use diesel::dsl::sql;
use diesel::expression::SqlLiteral;
use diesel::prelude::*;
use diesel::sql_types::{Array, BigInt, Text};
use diesel_async::RunQueryDsl;
use std::fmt::Display;

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

pub async fn run(opts: Opts) -> anyhow::Result<()> {
    let mut conn = db::oneoff_connection()
        .await
        .context("Failed to establish database connection")?;

    let mut crate_names = opts.crate_names;
    crate_names.sort();

    let existing_crates = crates::table
        .inner_join(crate_downloads::table)
        .filter(crates::name.eq_any(&crate_names))
        .select(CrateInfo::as_select())
        .load::<CrateInfo>(&mut conn)
        .await
        .context("Failed to look up crate name from the database")?;

    println!("Deleting the following crates:");
    println!();
    for name in &crate_names {
        match existing_crates.iter().find(|info| info.name == *name) {
            Some(info) => println!(" - {} ({info})", name.bold()),
            None => println!(" - {name} (⚠️ crate not found)"),
        }
    }
    println!();

    if !opts.yes && !dialoguer::confirm("Do you want to permanently delete these crates?").await? {
        return Ok(());
    }

    for name in &crate_names {
        if let Some(crate_info) = existing_crates.iter().find(|info| info.name == *name) {
            let id = crate_info.id;

            info!("{name}: Deleting crate from the database…");
            if let Err(error) = diesel::delete(crates::table.find(id))
                .execute(&mut conn)
                .await
            {
                warn!(%id, "{name}: Failed to delete crate from the database: {error}");
            }
        } else {
            info!("{name}: Skipped missing crate");
        };

        info!("{name}: Enqueuing index sync jobs…");
        let job = jobs::SyncToGitIndex::new(name);
        if let Err(error) = job.async_enqueue(&mut conn).await {
            warn!("{name}: Failed to enqueue SyncToGitIndex job: {error}");
        }

        let job = jobs::SyncToSparseIndex::new(name);
        if let Err(error) = job.async_enqueue(&mut conn).await {
            warn!("{name}: Failed to enqueue SyncToSparseIndex job: {error}");
        }

        info!("{name}: Enqueuing DeleteCrateFromStorage job…");
        let job = jobs::DeleteCrateFromStorage::new(name.into());
        if let Err(error) = job.async_enqueue(&mut conn).await {
            warn!("{name}: Failed to enqueue DeleteCrateFromStorage job: {error}");
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Queryable, Selectable)]
struct CrateInfo {
    #[diesel(select_expression = crates::columns::name)]
    name: String,
    #[diesel(select_expression = crates::columns::id)]
    id: i32,
    #[diesel(select_expression = crate_downloads::columns::downloads)]
    downloads: i64,
    #[diesel(select_expression = owners_subquery())]
    owners: Vec<String>,
    #[diesel(select_expression = rev_deps_subquery())]
    rev_deps: i64,
}

impl Display for CrateInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let id = self.id;
        let owners = self.owners.join(", ");

        write!(f, "id={id}, owners={owners}")?;
        if self.downloads > 5000 {
            let downloads = format!("downloads={}", self.downloads).bright_red().bold();
            write!(f, ", {downloads}")?;
        }
        if self.rev_deps > 0 {
            let rev_deps = format!("rev_deps={}", self.rev_deps).bright_red().bold();
            write!(f, ", {rev_deps}")?;
        }

        Ok(())
    }
}

/// A subquery that returns the owners of a crate as an array of strings.
#[diesel::dsl::auto_type]
fn owners_subquery() -> SqlLiteral<Array<Text>> {
    sql(r#"
        ARRAY(
            SELECT
                CASE WHEN crate_owners.owner_kind = 1 THEN
                    teams.login
                ELSE
                    users.gh_login
                END
            FROM crate_owners
            LEFT JOIN teams ON teams.id = crate_owners.owner_id
            LEFT JOIN users ON users.id = crate_owners.owner_id
            WHERE crate_owners.crate_id = crates.id
        )
    "#)
}

/// A subquery that returns the number of reverse dependencies of a crate.
///
/// **Warning:** this is an incorrect reverse dependencies query, since it
/// includes the `dependencies` rows for all versions, not just the
/// "default version" per crate. However, it's good enough for our
/// purposes here.
#[diesel::dsl::auto_type]
fn rev_deps_subquery() -> SqlLiteral<BigInt> {
    sql(r#"
       (
            SELECT COUNT(*)
            FROM dependencies
            WHERE dependencies.crate_id = crates.id
        )
    "#)
}
