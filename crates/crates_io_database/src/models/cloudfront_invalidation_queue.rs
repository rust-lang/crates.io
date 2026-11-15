use crate::schema::cloudfront_invalidation_queue;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

/// CloudFront distribution identifier.
///
/// Used to route invalidation requests to the correct CloudFront distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, diesel::types::Enum)]
#[diesel(sql_type = diesel::sql_types::Text, rename_all = "lowercase")]
pub enum CloudFrontDistribution {
    /// The index.crates.io distribution (sparse index metadata)
    Index,
    /// The static.crates.io distribution (crate files, readmes, etc.)
    Static,
}

impl CloudFrontDistribution {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Index => "index",
            Self::Static => "static",
        }
    }
}

#[derive(Debug, Identifiable, HasQuery, QueryableByName)]
#[diesel(table_name = cloudfront_invalidation_queue)]
pub struct CloudFrontInvalidationQueueItem {
    pub id: i64,
    pub path: String,
    pub distribution: CloudFrontDistribution,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = cloudfront_invalidation_queue, check_for_backend(diesel::pg::Pg))]
pub struct NewCloudFrontInvalidationQueueItem<'a> {
    pub path: &'a str,
    pub distribution: CloudFrontDistribution,
}

impl CloudFrontInvalidationQueueItem {
    /// Queues multiple invalidation paths for later processing.
    pub async fn queue_paths(
        mut conn: &AsyncPgConnection,
        distribution: CloudFrontDistribution,
        paths: &[String],
    ) -> QueryResult<usize> {
        let new_items: Vec<_> = paths
            .iter()
            .map(|path| NewCloudFrontInvalidationQueueItem { path, distribution })
            .collect();

        diesel::insert_into(cloudfront_invalidation_queue::table)
            .values(&new_items)
            .execute(&mut conn)
            .await
    }

    /// Fetches the oldest paths from the queue for a specific distribution.
    pub async fn fetch_batch(
        mut conn: &AsyncPgConnection,
        distribution: CloudFrontDistribution,
        limit: i64,
    ) -> QueryResult<Vec<CloudFrontInvalidationQueueItem>> {
        // Fetch the oldest entries up to the limit
        Self::query()
            .filter(cloudfront_invalidation_queue::distribution.eq(distribution))
            .order(cloudfront_invalidation_queue::created_at.asc())
            .limit(limit)
            .load(&mut conn)
            .await
    }

    /// Removes queue items by their IDs.
    pub async fn remove_items(
        mut conn: &AsyncPgConnection,
        item_ids: &[i64],
    ) -> QueryResult<usize> {
        diesel::delete(cloudfront_invalidation_queue::table)
            .filter(cloudfront_invalidation_queue::id.eq_any(item_ids))
            .execute(&mut conn)
            .await
    }
}
