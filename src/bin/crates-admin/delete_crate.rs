use crate::dialoguer;
use anyhow::{Context, anyhow, ensure};
use chrono::{DateTime, Utc};
use colored::Colorize;
use crates_io::controllers::krate::delete::max_downloads;
use crates_io::models::{NewDeletedCrate, User};
use crates_io::schema::{crate_downloads, deleted_crates};
use crates_io::worker::jobs;
use crates_io::{db, schema::crates};
use crates_io_database::schema::dependencies;
use crates_io_worker::BackgroundJob;
use diesel::dsl::{count_star, sql};
use diesel::expression::SqlLiteral;
use diesel::prelude::*;
use diesel::sql_types::{Array, Text};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
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

    /// Your GitHub username.
    #[arg(long)]
    deleted_by: String,

    /// An optional message explaining why the crate was deleted.
    #[arg(long)]
    message: Option<String>,

    /// The amount of time (in hours) before making the crate available
    /// for re-registration.
    #[arg(long, default_value = "24")]
    availability_delay: i64,
}

pub async fn run(opts: Opts) -> anyhow::Result<()> {
    let mut conn = db::oneoff_connection()
        .await
        .context("Failed to establish database connection")?;

    let mut crate_names = opts.crate_names;
    crate_names.sort();

    let existing_crates = CrateInfo::query()
        .filter(crates::name.eq_any(&crate_names))
        .load(&mut conn)
        .await
        .context("Failed to look up crate name from the database")?;

    let mut deleted_by_candidates = User::find_all_by_login(&mut conn, &opts.deleted_by)
        .await
        .context("Failed to look up `--deleted-by` user from the database")?
        .into_iter();
    let deleted_by = deleted_by_candidates
        .next()
        .ok_or_else(|| anyhow!("`--deleted-by` user not found"))?;
    ensure!(
        deleted_by_candidates.next().is_none(),
        "Expected only one `--deleted-by` user found but got more"
    );

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

    let now = Utc::now();
    let available_at = now + chrono::TimeDelta::hours(opts.availability_delay);

    for name in &crate_names {
        if let Some(crate_info) = existing_crates.iter().find(|info| info.name == *name) {
            let id = crate_info.id;

            let deleted_crate = NewDeletedCrate::builder(name)
                .created_at(&crate_info.created_at)
                .deleted_at(&now)
                .deleted_by(deleted_by.id)
                .maybe_message(opts.message.as_deref())
                .available_at(&available_at)
                .build();

            info!("{name}: Deleting crate from the database…");
            let result = conn
                .transaction(|conn| delete_from_database(conn, id, deleted_crate).scope_boxed())
                .await;

            if let Err(error) = result {
                warn!(%id, "{name}: Failed to delete crate from the database: {error}");
            };
        } else {
            info!("{name}: Skipped missing crate");
        };

        info!("{name}: Enqueuing background jobs…");
        let git_index_job = jobs::SyncToGitIndex::new(name);
        let sparse_index_job = jobs::SyncToSparseIndex::new(name);
        let delete_from_storage_job = jobs::DeleteCrateFromStorage::new(name.into());

        if let Err(error) = tokio::try_join!(
            git_index_job.enqueue(&mut conn),
            sparse_index_job.enqueue(&mut conn),
            delete_from_storage_job.enqueue(&mut conn),
        ) {
            warn!("{name}: Failed to enqueue background job: {error}");
        }
    }

    Ok(())
}

async fn delete_from_database(
    conn: &mut AsyncPgConnection,
    crate_id: i32,
    deleted_crate: NewDeletedCrate<'_>,
) -> anyhow::Result<()> {
    diesel::delete(crates::table.find(crate_id))
        .execute(conn)
        .await?;

    diesel::insert_into(deleted_crates::table)
        .values(deleted_crate)
        .execute(conn)
        .await?;

    Ok(())
}

#[derive(Debug, Clone, HasQuery)]
#[diesel(
    base_query = crates::table
        .inner_join(crate_downloads::table)
)]
struct CrateInfo {
    #[diesel(select_expression = crates::columns::name)]
    name: String,
    #[diesel(select_expression = crates::columns::id)]
    id: i32,
    #[diesel(select_expression = crates::columns::created_at)]
    created_at: DateTime<Utc>,
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

        let mut downloads = format!("downloads={}", self.downloads);
        let age = Utc::now().signed_duration_since(self.created_at);
        if self.downloads as u64 > max_downloads(&age) {
            downloads.push_str("🚨🚨🚨");
        }
        write!(f, ", {downloads}")?;

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
            AND crate_owners.deleted = 'f'
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
fn rev_deps_subquery() -> _ {
    dependencies::table
        .select(count_star())
        .filter(dependencies::crate_id.eq(crates::id))
        .single_value()
        .assume_not_null()
}
