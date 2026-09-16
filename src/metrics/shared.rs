use super::consts::{
    DB_CLIENT_CONNECTION_COUNT, DB_CLIENT_CONNECTION_POOL_NAME, DB_CLIENT_CONNECTION_STATE,
};
use super::otel::kv;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::deadpool::Pool;
use opentelemetry::metrics::Meter;

/// OpenTelemetry instruments shared by the server and background worker.
#[derive(Clone, Debug)]
pub struct SharedMetrics {
    meter: Meter,
}

impl SharedMetrics {
    /// Creates the shared instruments from `meter`.
    pub fn new(meter: &Meter) -> Self {
        Self {
            meter: meter.clone(),
        }
    }

    /// Tracks connection counts for `pool`.
    pub fn track_db_pool(&self, name: &'static str, pool: &Pool<AsyncPgConnection>) {
        let pool = pool.clone();
        self.meter
            .i64_observable_up_down_counter(DB_CLIENT_CONNECTION_COUNT)
            .with_unit("{connection}")
            .with_callback(move |observer| {
                let status = pool.status();

                let idle_connections = status.available as i64;
                let idle_attributes = [
                    kv(DB_CLIENT_CONNECTION_POOL_NAME, name),
                    kv(DB_CLIENT_CONNECTION_STATE, "idle"),
                ];
                observer.observe(idle_connections, &idle_attributes);

                let used_connections = (status.size - status.available) as i64;
                let used_attributes = [
                    kv(DB_CLIENT_CONNECTION_POOL_NAME, name),
                    kv(DB_CLIENT_CONNECTION_STATE, "used"),
                ];
                observer.observe(used_connections, &used_attributes);
            })
            .build();
    }
}
