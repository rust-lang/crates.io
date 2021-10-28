use crate::models::Crate;
use crate::{db, schema::crates};
use anyhow::Context;
use diesel::prelude::*;

#[derive(clap::Parser, Debug)]
#[clap(
    name = "fill-top-versions",
    about = "Iterates over every crate ever uploaded and updates their \
        `highest_version`, `highest_stable_version` and `newest_version` columns.",
    after_help = "Warning: this can take a lot of time."
)]
pub struct Opts {
    /// How many crates should be queried and processed at a time.
    #[clap(long, default_value = "25")]
    page_size: u32,
}

pub fn run(opts: Opts) -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let conn = db::connect_now().context("Failed to connect to the database")?;

    let num_crates: i64 = crates::table
        .count()
        .get_result(&conn)
        .context("Failed to count crates in the database")?;

    let page_size = opts.page_size as i64;
    let pages = num_crates / page_size + 1;

    for page in 1..=pages {
        info!("Processing page {} / {}", page, pages);

        let crates: Vec<Crate> = Crate::all()
            .order(crates::id.asc())
            .limit(page_size)
            .offset(page_size * (page - 1))
            .load(&conn)
            .context("Failed to load crates")?;

        conn.transaction::<_, anyhow::Error, _>(|| {
            diesel::sql_query("ALTER TABLE crates DISABLE TRIGGER trigger_crates_set_updated_at")
                .execute(&conn)
                .context("Failed to disable the `set_updated_at` trigger")?;

            for krate in crates {
                let top_versions = krate
                    .top_versions(&conn)
                    .context("Failed to calculate top versions")?;

                let highest = top_versions.highest.as_ref();
                let highest = highest.map(|it| it.to_string()).unwrap_or_default();

                let highest_stable = top_versions.highest_stable.as_ref();
                let highest_stable = highest_stable.map(|it| it.to_string()).unwrap_or_default();

                let newest = top_versions.newest.as_ref();
                let newest = newest.map(|it| it.to_string()).unwrap_or_default();

                info!(
                    crate = %krate.name,
                    highest = %highest,
                    highest_stable = %highest_stable,
                    newest = %newest,
                    "Saving top versions",
                );
                krate
                    .update_top_versions(&conn, &top_versions)
                    .context("Failed to update top versions in the database")?;
            }

            diesel::sql_query("ALTER TABLE crates ENABLE TRIGGER trigger_crates_set_updated_at")
                .execute(&conn)
                .context("Failed to enable the `set_updated_at` trigger")?;

            Ok(())
        })?;
    }

    Ok(())
}
