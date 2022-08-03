use chrono::NaiveDateTime;

use diesel::pg::Pg;
use diesel::prelude::*;

use url::Url;

use crate::app::App;
use crate::schema::webhooks;
use crate::util::errors::{cargo_err, AppResult};

#[derive(Debug, Queryable, Identifiable, Associations)]
#[table_name = "webhooks"]
#[belongs_to(User, foreign_key = "owner_id")]
#[belongs_to(Crate, foreign_key = "crate_id")]
pub struct Webhook {
    pub id: i32,
    pub owner_id: i32,
    pub crate_id: i32,
    pub webhook_url: Url,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
    pub deleted: bool,
}

pub struct NewWebhook {
    pub webhook_url: Url
}

impl NewWebhook {
    pub fn create_or_update(
        self,
        conn: &PgConnection,
    )
}