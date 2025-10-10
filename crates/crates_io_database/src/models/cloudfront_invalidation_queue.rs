use crate::schema::cloudfront_invalidation_queue;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

#[derive(Debug, Identifiable, Queryable, QueryableByName, Selectable)]
#[diesel(table_name = cloudfront_invalidation_queue, check_for_backend(diesel::pg::Pg))]
pub struct CloudFrontInvalidationQueueItem {
    pub id: i64,
    pub path: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = cloudfront_invalidation_queue, check_for_backend(diesel::pg::Pg))]
pub struct NewCloudFrontInvalidationQueueItem<'a> {
    pub path: &'a str,
}

impl CloudFrontInvalidationQueueItem {
    /// Queue multiple invalidation paths for later processing
    pub async fn queue_paths(conn: &mut AsyncPgConnection, paths: &[String]) -> QueryResult<usize> {
        let new_items: Vec<_> = paths
            .iter()
            .map(|path| NewCloudFrontInvalidationQueueItem { path })
            .collect();

        diesel::insert_into(cloudfront_invalidation_queue::table)
            .values(&new_items)
            .execute(conn)
            .await
    }

    /// Fetch the oldest paths from the queue
    pub async fn fetch_batch(
        conn: &mut AsyncPgConnection,
        limit: i64,
    ) -> QueryResult<Vec<CloudFrontInvalidationQueueItem>> {
        // Fetch the oldest entries up to the limit
        cloudfront_invalidation_queue::table
            .order(cloudfront_invalidation_queue::created_at.asc())
            .limit(limit)
            .select(Self::as_select())
            .load(conn)
            .await
    }

    /// Remove queue items by their IDs
    pub async fn remove_items(
        conn: &mut AsyncPgConnection,
        item_ids: &[i64],
    ) -> QueryResult<usize> {
        diesel::delete(cloudfront_invalidation_queue::table)
            .filter(cloudfront_invalidation_queue::id.eq_any(item_ids))
            .execute(conn)
            .await
    }
}
