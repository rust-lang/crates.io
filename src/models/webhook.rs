use chrono::NaiveDateTime;

use diesel::pg::Pg;
use diesel::prelude::*;

use crate::models::{Crate, User};
use crate::schema::webhooks;

use url::Url;
/// The model representing a row in the `users` database table.
#[derive(Clone, Debug, PartialEq, Eq, Queryable, Identifiable, Associations)]
#[table_name = "webhooks"]
#[belongs_to(User, foreign_key = "owner_id")]
// #[belongs_to(Crate, foreign_key = "crate_id")]
pub struct Webhook {
    pub id: i32,
    pub owner_id: i32,
    // pub crate_id: i32,
    pub webhook_url: Url,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Represents a new webhook record insertable to the `webhooks` table
#[derive(Insertable, Debug, Default)]
#[table_name = "webhooks"]
pub struct NewWebhook {
    pub owner_id: i32,
    // pub crate_id: i32,
    pub webhook_url: String,
}

impl NewWebhook {}

enum CrateWebhookEvent {
    Version(VersionAction),
    Owners(OwnersAction),
}

enum VersionAction {
    Published,
    Yanked,
}

enum OwnersAction {
    Added,
    Removed,
    Invited,
    InviteRevoked,
}
