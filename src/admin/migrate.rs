use anyhow::Error;

static CATEGORIES_TOML: &'static str = include_str!("../boot/categories.toml");
diesel_migrations::embed_migrations!("./migrations");

#[derive(clap::Clap, Debug, Copy, Clone)]
#[clap(name = "migrate", about = "Migrate the database.")]
pub struct Opts;

pub fn run(_opts: Opts) -> Result<(), Error> {
    println!("==> migrating the database");
    let conn = crate::db::connect_now()?;
    embedded_migrations::run_with_output(&conn, &mut std::io::stdout())?;

    println!("==> synchronizing crate categories");
    crate::boot::categories::sync(CATEGORIES_TOML).unwrap();

    Ok(())
}
