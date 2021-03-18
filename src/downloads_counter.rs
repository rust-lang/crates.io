use crate::App;
use anyhow::Error;
use dashmap::{DashMap, SharedValue};
use diesel::{pg::upsert::excluded, prelude::*};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};

/// crates.io receives a lot of download requests, and we can't execute a write query to the
/// database during each connection for performance reasons. To reduce the write load, this struct
/// collects the pending updates from the current process and writes in batch.
///
/// To avoid locking the whole data structure behind a RwLock, which could potentially delay
/// requests, this uses the dashmap crate. A DashMap has the same public API as an HashMap, but
/// stores the items into `num_cpus()*4` individually locked shards. This approach reduces the
/// likelyhood of a request encountering a locked shard.
///
/// Persisting the download counts in the database also takes advantage of the inner sharding of
/// DashMaps: to avoid locking all the download requests at the same time each iteration only
/// persists a single shard at the time.
///
/// The disadvantage of this approach is that download counts are stored in memory until they're
/// persisted, so it's possible to lose some of them if the process exits ungracefully. While
/// that's far from ideal, the advantage of batching database updates far outweights potentially
/// losing some download counts.
#[derive(Debug)]
pub struct DownloadsCounter {
    /// Inner storage for the download counts.
    inner: DashMap<i32, AtomicUsize>,
    /// Index of the next shard that should be persisted by `persist_next_shard`.
    shard_idx: AtomicUsize,
    /// Number of downloads that are not yet persisted on the database. This is just used as a
    /// metric included in log lines, and it's not guaranteed to be accurate.
    pending_count: AtomicI64,
}

impl DownloadsCounter {
    pub(crate) fn new() -> Self {
        Self {
            inner: DashMap::new(),
            shard_idx: AtomicUsize::new(0),
            pending_count: AtomicI64::new(0),
        }
    }

    pub(crate) fn increment(&self, version_id: i32) {
        self.pending_count.fetch_add(1, Ordering::SeqCst);

        if let Some(counter) = self.inner.get(&version_id) {
            // The version is already recorded in the DashMap, so we don't need to lock the whole
            // shard in write mode. The shard is instead locked in read mode, which allows an
            // unbounded number of readers as long as there are no write locks.
            counter.value().fetch_add(1, Ordering::SeqCst);
        } else {
            // The version is not in the DashMap, so we need to lock the whole shard in write mode
            // and insert the version into it. This has worse performance than the above case.
            self.inner
                .entry(version_id)
                .and_modify(|counter| {
                    // Handle the version being inserted by another thread while we were waiting
                    // for the write lock on the shard.
                    counter.fetch_add(1, Ordering::SeqCst);
                })
                .or_insert_with(|| AtomicUsize::new(1));
        }
    }

    pub fn persist_all_shards(&self, app: &App) -> Result<(), Error> {
        let conn = app.primary_database.get()?;

        let mut counted_downloads = 0;
        let mut counted_versions = 0;
        let mut pending_downloads = 0;
        for shard in self.inner.shards() {
            let shard = std::mem::take(&mut *shard.write());
            let stats = self.persist_shard(&conn, shard)?;

            counted_downloads += stats.counted_downloads;
            counted_versions += stats.counted_versions;
            pending_downloads = stats.pending_downloads;
        }

        println!(
            "downloads_counter all_shards counted_versions={} counted_downloads={} pending_downloads={}",
            counted_versions,
            counted_downloads,
            pending_downloads,
        );

        Ok(())
    }

    pub fn persist_next_shard(&self, app: &App) -> Result<(), Error> {
        let conn = app.primary_database.get()?;

        // Replace the next shard in the ring with an empty HashMap (clearing it), and return the
        // previous contents for processing. The fetch_add method wraps around on overflow, so it's
        // fine to keep incrementing it without resetting.
        let shards = self.inner.shards();
        let idx = self.shard_idx.fetch_add(1, Ordering::SeqCst) % shards.len();
        let shard = std::mem::take(&mut *shards[idx].write());

        let stats = self.persist_shard(&conn, shard)?;
        println!(
            "downloads_counter shard={} counted_versions={} counted_downloads={} pending_downloads={}",
            idx,
            stats.counted_versions,
            stats.counted_downloads,
            stats.pending_downloads,
        );

        Ok(())
    }

    fn persist_shard(
        &self,
        conn: &PgConnection,
        shard: HashMap<i32, SharedValue<AtomicUsize>>,
    ) -> Result<PersistStats, Error> {
        use crate::schema::version_downloads::dsl::*;

        let mut counted_downloads = 0;
        let mut counted_versions = 0;
        let mut to_insert = Vec::new();
        for (key, atomic) in shard.iter() {
            let count = atomic.get().load(Ordering::SeqCst);
            counted_downloads += count;
            counted_versions += 1;

            to_insert.push((*key, count));
        }

        if !to_insert.is_empty() {
            // The rows we're about to insert need to be sorted to avoid deadlocks when multiple
            // instances of crates.io are running at the same time.
            //
            // In PostgreSQL a transaction modifying a row locks that row until the transaction is
            // committed. Multiple transactions inserting rows into a table could end up
            // deadlocking each other though: PostgreSQL will detect that deadlock, abort one of
            // the transactions and allow the other one to continue. We don't want that to happen,
            // as we'd lose the downloads from the aborted transaction.
            //
            // Ensuring the rows are inserted in a consistent order (in our case by sorting them by
            // the version ID) will prevent deadlocks from occuring. For more information:
            //
            //     https://www.postgresql.org/docs/11/explicit-locking.html#LOCKING-DEADLOCKS
            //
            to_insert.sort_by_key(|(key, _)| *key);

            let to_insert = to_insert
                .into_iter()
                .map(|(key, count)| (version_id.eq(key), downloads.eq(count as i32)))
                .collect::<Vec<_>>();

            diesel::insert_into(version_downloads)
                .values(&to_insert)
                .on_conflict((version_id, date))
                .do_update()
                .set(downloads.eq(downloads + excluded(downloads)))
                .execute(conn)?;
        }

        let old_pending = self
            .pending_count
            .fetch_sub(counted_downloads as i64, Ordering::SeqCst);

        Ok(PersistStats {
            counted_downloads,
            counted_versions,
            pending_downloads: old_pending - counted_downloads as i64,
        })
    }

    pub fn shards_count(&self) -> usize {
        self.inner.shards().len()
    }
}

struct PersistStats {
    counted_downloads: usize,
    counted_versions: usize,
    pending_downloads: i64,
}
