use diesel::prelude::*;
use diesel_migrations::run_pending_migrations;
use std::env;
use std::error::Error;

fn main() {
    if let Err(err) = load_git_commit() {
        println!("cargo:warning=failed to load git commit: {}", err);
    }

    println!("cargo:rerun-if-env-changed=TEST_DATABASE_URL");
    println!("cargo:rerun-if-changed=.env");
    println!("cargo:rerun-if-changed=migrations/");
    if env::var("PROFILE") == Ok("debug".into()) {
        if let Ok(database_url) = dotenv::var("TEST_DATABASE_URL") {
            let connection = PgConnection::establish(&database_url)
                .expect("Could not connect to TEST_DATABASE_URL");
            run_pending_migrations(&connection).expect("Error running migrations");
        }
    }
}

fn load_git_commit() -> Result<(), Box<dyn Error>> {
    let repo = git2::Repository::open(env::current_dir()?)?;
    let head = repo.head()?;

    // Ensure the build script is re-run if the current commit changes.
    println!("cargo:rerun-if-changed=.git/HEAD");
    if let Some(name) = head.name() {
        println!("cargo:rerun-if-changed=.git/{}", name);
    }

    if let Some(hash) = head.target() {
        let mut hash = hash.to_string();
        hash.truncate(7);
        println!("cargo:rustc-env=CRATES_IO_GIT_COMMIT={}", hash);
    }

    Ok(())
}
