mod sync_crate_feed;
mod sync_crates_feed;
mod sync_updates_feed;

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
