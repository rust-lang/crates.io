use diesel::prelude::*;
use diesel_migrations::{FileBasedMigrations, MigrationHarness};
use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=TEST_DATABASE_URL");
    println!("cargo:rerun-if-changed=.env");
    println!("cargo:rerun-if-changed=migrations/");
    if env::var("PROFILE") == Ok("debug".into()) {
        if let Ok(database_url) = dotenvy::var("TEST_DATABASE_URL") {
            let connection = &mut PgConnection::establish(&database_url)
                .expect("Could not connect to TEST_DATABASE_URL");
            let migrations = FileBasedMigrations::find_migrations_directory()
                .expect("Could not find migrations");
            connection
                .run_pending_migrations(migrations)
                .expect("Error running migrations");
        }
    }
}
