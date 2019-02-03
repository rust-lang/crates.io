use diesel::prelude::*;
use diesel_migrations::run_pending_migrations;
use dotenv::dotenv;
use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=TEST_DATABASE_URL");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=migrations/");
    if env::var("PROFILE") == Ok("debug".into()) {
        let _ = dotenv();
        if let Ok(database_url) = env::var("TEST_DATABASE_URL") {
            let connection = PgConnection::establish(&database_url)
                .expect("Could not connect to TEST_DATABASE_URL");
            run_pending_migrations(&connection).expect("Error running migrations");
        }
    }
}
