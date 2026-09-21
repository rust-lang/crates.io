mod sync_crate_feed;
mod sync_crates_feed;
mod sync_updates_feed;

use crate::storage::StorageKey;
use crate::worker::WorkerContext;
use crates_io_database::models::CloudFrontDistribution;
use diesel_async::AsyncPgConnection;
use tracing::{info, warn};

pub use sync_crate_feed::SyncCrateFeed;
pub use sync_crates_feed::SyncCratesFeed;
pub use sync_updates_feed::SyncUpdatesFeed;

/// Serializes an RSS channel into a pretty-printed XML byte buffer.
fn serialize_channel(channel: &rss::Channel) -> anyhow::Result<Vec<u8>> {
    let mut buffer = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buffer);
    channel.pretty_write_to(&mut cursor, b' ', 4)?;
    Ok(buffer)
}

/// Uploads an RSS channel, then attempts to invalidate its cached copies.
///
/// Upload failures are returned. CDN invalidation failures are logged without
/// failing publication of the uploaded feed.
async fn publish_channel(
    ctx: &WorkerContext,
    conn: &AsyncPgConnection,
    key: &StorageKey<'_>,
    channel: &rss::Channel,
) -> anyhow::Result<()> {
    let path = key.path();

    info!("Uploading feed to storage…");
    let bytes = serialize_channel(channel)?;
    ctx.storage.upload(key, bytes.into()).await?;

    let dist = CloudFrontDistribution::Static;
    if let Err(error) = ctx.invalidate_cdns(conn, dist, path.as_ref()).await {
        warn!("Failed to invalidate CDN caches: {error}");
    }

    Ok(())
}
