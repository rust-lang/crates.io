use bon::Builder;
use chrono::{DateTime, Utc};
use crates_io_database::schema::deleted_crates;

/// Struct used to `INSERT` a new `deleted_crates` record into the database.
#[derive(Insertable, Debug, Builder)]
#[diesel(table_name = deleted_crates, check_for_backend(diesel::pg::Pg))]
pub struct NewDeletedCrate<'a> {
    #[builder(start_fn)]
    name: &'a str,
    created_at: &'a DateTime<Utc>,
    deleted_at: &'a DateTime<Utc>,
    deleted_by: Option<i32>,
    message: Option<&'a str>,
    available_at: &'a DateTime<Utc>,
    min_version: Option<&'a str>,
}
