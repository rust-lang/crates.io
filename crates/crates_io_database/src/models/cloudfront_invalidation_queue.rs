use crate::schema::cloudfront_invalidation_queue;
use diesel::AsExpression;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::Text;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use std::io::Write;
use std::str::FromStr;

/// CloudFront distribution identifier.
///
/// Used to route invalidation requests to the correct CloudFront distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = Text)]
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

impl FromStr for CloudFrontDistribution {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "index" => Ok(Self::Index),
            "static" => Ok(Self::Static),
            _ => Err(format!("Unknown CloudFront distribution: {s}")),
        }
    }
}

impl ToSql<Text, Pg> for CloudFrontDistribution {
    fn to_sql(&self, out: &mut Output<'_, '_, Pg>) -> serialize::Result {
        out.write_all(self.as_str().as_bytes())?;
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Pg> for CloudFrontDistribution {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> deserialize::Result<Self> {
        let value = <String as FromSql<Text, Pg>>::from_sql(bytes)?;
        Ok(value.parse()?)
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
    /// Queue multiple invalidation paths for later processing
    pub async fn queue_paths(
        conn: &mut AsyncPgConnection,
        distribution: CloudFrontDistribution,
        paths: &[String],
    ) -> QueryResult<usize> {
        let new_items: Vec<_> = paths
            .iter()
            .map(|path| NewCloudFrontInvalidationQueueItem { path, distribution })
            .collect();

        diesel::insert_into(cloudfront_invalidation_queue::table)
            .values(&new_items)
            .execute(conn)
            .await
    }

    /// Fetch the oldest paths from the queue for a specific distribution
    pub async fn fetch_batch(
        conn: &mut AsyncPgConnection,
        distribution: CloudFrontDistribution,
        limit: i64,
    ) -> QueryResult<Vec<CloudFrontInvalidationQueueItem>> {
        // Fetch the oldest entries up to the limit
        Self::query()
            .filter(cloudfront_invalidation_queue::distribution.eq(distribution))
            .order(cloudfront_invalidation_queue::created_at.asc())
            .limit(limit)
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
