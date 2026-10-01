mod sync_crate_feed;
mod sync_crates_feed;
mod sync_updates_feed;

use crate::storage::StorageKey;
use crate::worker::WorkerContext;
use crates_io_database::models::CloudFrontDistribution;
use diesel_async::AsyncPgConnection;
use rss::extension::atom::{AtomExtension, Link};
use rss::extension::{Extension, ExtensionMap};
use tracing::{info, warn};

pub use sync_crate_feed::SyncCrateFeed;
pub use sync_crates_feed::SyncCratesFeed;
pub use sync_updates_feed::SyncUpdatesFeed;

/// Creates a channel with shared crates.io feed metadata.
fn channel_defaults(feed_url: String) -> rss::Channel {
    let link = Link {
        href: feed_url,
        rel: "self".to_string(),
        mime_type: Some("application/rss+xml".to_string()),
        ..Default::default()
    };

    rss::Channel {
        language: Some("en".to_string()),
        atom_ext: Some(AtomExtension { links: vec![link] }),
        namespaces: [("crates".to_string(), "https://crates.io/".to_string())].into(),
        ..Default::default()
    }
}

/// Builds RSS extensions in the crates.io namespace.
fn crates_extensions(fields: impl IntoIterator<Item = (&'static str, String)>) -> ExtensionMap {
    let extensions = fields
        .into_iter()
        .map(|(name, value)| (name.to_string(), vec![crates_extension(name, value)]))
        .collect();

    [("crates".to_string(), extensions)].into()
}

/// Builds an RSS element in the crates.io namespace.
fn crates_extension(name: &str, value: String) -> Extension {
    Extension {
        name: format!("crates:{name}"),
        value: Some(value),
        ..Default::default()
    }
}

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
        warn!("Failed to invalidate CDN caches: {error:#}");
    }

    Ok(())
}
