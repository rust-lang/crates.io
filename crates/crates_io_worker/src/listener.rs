use diesel_async::pooled_connection::deadpool::{Object, Pool};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use futures_util::StreamExt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// The Postgres `LISTEN`/`NOTIFY` channel that a database trigger signals
/// whenever a new background job is inserted.
pub const CHANNEL: &str = "background_jobs";

/// How long to wait before the first reconnect attempt after the listener
/// connection was lost. Subsequent failures back off exponentially up to
/// [`MAX_RECONNECT_DELAY`].
const INITIAL_RECONNECT_DELAY: Duration = Duration::from_secs(1);

/// The upper bound for the exponential reconnect backoff, so that a sustained
/// outage does not stop the listener from retrying entirely.
const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(60);

/// Listen for `NOTIFY` messages on the [`CHANNEL`] and wake up the workers
/// whenever a new job is enqueued.
///
/// This keeps a dedicated connection open for the lifetime of the runner. If
/// the connection is lost, it is re-established automatically, backing off
/// exponentially while reconnect attempts keep failing.
pub async fn listen_for_new_jobs(pool: Pool<AsyncPgConnection>, notify: Arc<Notify>) {
    let mut delay = INITIAL_RECONNECT_DELAY;
    loop {
        if let Err(error) = listen(&pool, &notify, &mut delay).await {
            warn!("Background job listener failed, reconnecting in {delay:?}: {error}");
        }
        sleep(delay).await;
        delay = (delay * 2).min(MAX_RECONNECT_DELAY);
    }
}

async fn listen(
    pool: &Pool<AsyncPgConnection>,
    notify: &Notify,
    delay: &mut Duration,
) -> anyhow::Result<()> {
    // Take the connection out of the pool so that it stays dedicated to
    // listening and is never recycled for regular queries.
    let mut conn = Object::take(pool.get().await?);

    diesel::sql_query(format!("LISTEN {CHANNEL}"))
        .execute(&mut conn)
        .await?;

    // The connection is up and listening, so reset the backoff for the next
    // reconnect regardless of how this session eventually ends.
    *delay = INITIAL_RECONNECT_DELAY;

    info!("Listening for new background jobs…");

    let mut stream = std::pin::pin!(conn.notifications_stream());
    while let Some(notification) = stream.next().await {
        // Surface per-notification errors so the caller reconnects. A cleanly
        // closed connection instead ends the stream and reconnects the same way.
        notification?;

        debug!("Received notification about a new background job.");
        notify.notify_waiters();
    }

    Ok(())
}
