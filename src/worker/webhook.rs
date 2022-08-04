use crate::background_jobs::Environment;
use crate::models::{CrateOwner, Webhook};
use crate::schema::{crate_owners, crates, versions, webhooks};
use diesel::prelude::*;
use serde_json::json;
use swirl::PerformError;

// What information do we want to send in the webhook?
// - NewVersionReleased
//  - Crate Name
//  - Crate Version
//  - Published by
//  - Published at
//  -

struct CrateWebhook {
    name: String,
    version: String,
}

#[swirl::background_job]
pub fn notify_owners(
    env: &Environment,
    conn: &PgConnection,
    krate_id: i32,
) -> Result<(), PerformError> {
    println!("Executing notify_owners job!");

    let crate_owners: Vec<i32> = crate_owners::table
        .select(crate_owners::owner_id)
        .filter(crate_owners::crate_id.eq(krate_id))
        .filter(crate_owners::owner_kind.eq(0))
        .load(conn)
        .expect("Error loading crate_owners");

    let client = env.http_client();

    for owner in crate_owners {
        let urls: Vec<String> = webhooks::table
            .select(webhooks::webhook_url)
            .filter(webhooks::owner_id.eq(owner))
            .load(conn)
            .expect("Error loading webhooks");

        let name: Vec<String> = crates::table
            .select(crates::name)
            .filter(crates::id.eq(krate_id))
            .load(conn)
            .expect("Error loading crates");

        for webhook_url in urls {
            client
                .post(webhook_url)
                .json(&json!(
                    {
                        "name": name.first().unwrap(),
                        "version": "a.b.c"
                    }
                ))
                .send()?;
        }
    }

    // client.post("<URL HERE>").json(&new_version).send()?;

    // println!("Executing notify_owners job!");
    Ok(())
}
