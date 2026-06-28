use crate::schema::cache_tags_backfills;

/// Struct used to `INSERT` a completed cache-tags backfill record into the
/// `cache_tags_backfills` table.
///
/// A row records that every S3 object for the crate has been re-copied with
/// `cache-tags` metadata. `crate_id` is `None` only if the crate was deleted
/// after the backfill completed but before the record was written.
#[derive(Debug, diesel::Insertable, bon::Builder)]
#[diesel(table_name = cache_tags_backfills, check_for_backend(diesel::pg::Pg))]
pub struct NewCacheTagsBackfillRow<'a> {
    pub crate_id: Option<i32>,
    pub crate_name: &'a str,
}
