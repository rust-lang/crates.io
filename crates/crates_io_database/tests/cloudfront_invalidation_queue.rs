use crates_io_database::models::CloudFrontDistribution::{Index, Static};
use crates_io_database::models::CloudFrontInvalidationQueueItem as Item;
use crates_io_database::schema::cloudfront_invalidation_queue as queue;
use crates_io_test_db::TestDatabase;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

/// Distributions are stored as lowercase text and only fetched for the requested distribution.
#[tokio::test]
async fn distribution_round_trip() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    let index_paths = ["/index-path".to_string()];
    Item::queue_paths(&conn, Index, &index_paths).await.unwrap();

    let static_paths = ["/static-path".to_string()];
    Item::queue_paths(&conn, Static, &static_paths)
        .await
        .unwrap();

    let stored: Vec<(String, String)> = queue::table
        .select((queue::path, queue::distribution))
        .order(queue::path)
        .load(&mut conn)
        .await
        .unwrap();

    let expected = [("/index-path", "index"), ("/static-path", "static")]
        .map(|(path, distribution)| (path.to_string(), distribution.to_string()));
    assert_eq!(stored, expected);

    let items = Item::fetch_batch(&conn, Static, 10).await.unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].path, "/static-path");
    assert_eq!(items[0].distribution, Static);
}
