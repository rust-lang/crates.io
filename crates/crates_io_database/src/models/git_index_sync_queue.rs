use crate::schema::git_index_sync_queue;
use diesel::{prelude::*, sql_types::Integer};
use diesel_async::{AsyncPgConnection, RunQueryDsl};

#[derive(Debug, HasQuery, QueryableByName)]
#[diesel(table_name = git_index_sync_queue)]
pub struct GitIndexSyncQueueItem {
    pub crate_name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = git_index_sync_queue, check_for_backend(diesel::pg::Pg))]
pub struct NewGitIndexSyncQueueItem<'a> {
    pub crate_name: &'a str,
}

impl GitIndexSyncQueueItem {
    /// Queue a crate to be synced.
    ///
    /// If the crate is already in the queue, then nothing happens, successfully.
    pub async fn queue(conn: &mut AsyncPgConnection, crate_name: &str) -> QueryResult<()> {
        diesel::insert_into(git_index_sync_queue::table)
            .values(NewGitIndexSyncQueueItem { crate_name })
            // It's possible the crate has already been enqueued, in which case we won't change
            // anything, since we want to keep the original creation time.
            .on_conflict_do_nothing()
            .execute(conn)
            .await?;

        Ok(())
    }

    /// Fetch the oldest crates from the queue, deleting them as we go.
    ///
    /// It's likely that you'll want to use this in a transaction so this can rolled back if
    /// downstream processing fails.
    pub async fn fetch_batch(
        conn: &mut AsyncPgConnection,
        limit: i32,
    ) -> QueryResult<Vec<GitIndexSyncQueueItem>> {
        diesel::sql_query(include_str!("git_index_sync_queue_fetch_batch.sql"))
            .bind::<Integer, _>(limit)
            .load(conn)
            .await
    }
}
