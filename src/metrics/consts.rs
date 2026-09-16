/// The instrumentation scope name used by crates.io metrics.
pub const METER_NAME: &str = "crates.io";

/// The queued background jobs instrument name.
pub const BACKGROUND_JOBS: &str = "crates_io.background_jobs";

/// The total crates instrument name.
pub const CRATES_TOTAL: &str = "crates_io.crates_total";

/// The database pool connection count instrument name.
pub const DB_CLIENT_CONNECTION_COUNT: &str = "db.client.connection.count";

/// The database connection fallback instrument name.
pub const DB_CLIENT_CONNECTION_FALLBACKS: &str = "crates_io.db.client.connection.fallbacks";

/// The database pool name attribute key.
pub const DB_CLIENT_CONNECTION_POOL_NAME: &str = "db.client.connection.pool.name";

/// The database connection state attribute key.
pub const DB_CLIENT_CONNECTION_STATE: &str = "db.client.connection.state";

/// The HTTP request method attribute key.
pub const HTTP_REQUEST_METHOD: &str = "http.request.method";

/// The active HTTP server requests instrument name.
pub const HTTP_SERVER_ACTIVE_REQUESTS: &str = "http.server.active_requests";

/// The background job type attribute key.
pub const JOB: &str = "job";

/// The background job priority attribute key.
pub const PRIORITY: &str = "priority";

/// The URL scheme attribute key.
pub const URL_SCHEME: &str = "url.scheme";

/// The total versions instrument name.
pub const VERSIONS_TOTAL: &str = "crates_io.versions_total";
