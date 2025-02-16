use crate::schema::deleted_crates;
use bon::Builder;
use chrono::{DateTime, Utc};

/// Struct used to `INSERT` a new `deleted_crates` record into the database.
#[derive(diesel::Insertable, Debug, Builder)]
#[diesel(table_name = deleted_crates, check_for_backend(diesel::pg::Pg))]
pub struct NewDeletedCrate<'a> {
    #[builder(start_fn)]
    name: &'a str,
    created_at: &'a DateTime<Utc>,
    deleted_at: &'a DateTime<Utc>,
    deleted_by: Option<i32>,
    message: Option<&'a str>,
    available_at: &'a DateTime<Utc>,
}
