/// The instrumentation scope name used by crates.io metrics.
pub const METER_NAME: &str = "crates.io";

/// The authentication check instrument name.
pub const AUTH_CHECKS: &str = "crates_io.auth.checks";

/// The required endpoint scope attribute key.
pub const AUTH_ENDPOINT_SCOPE: &str = "crates_io.auth.endpoint_scope";

/// The authentication type attribute key.
pub const AUTH_TYPE: &str = "crates_io.auth.type";

/// The queued background jobs instrument name.
pub const BACKGROUND_JOBS: &str = "crates_io.background_jobs";

/// The oldest outstanding background job age instrument name.
pub const BACKGROUND_JOBS_OLDEST_AGE: &str = "crates_io.background_jobs.oldest_age";

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

/// The database dump size instrument name.
pub const DB_DUMP_SIZE_BYTES: &str = "crates_io.db_dump_size_bytes";

/// The database dump upload duration instrument name.
pub const DB_DUMP_UPLOAD_DURATION_NS: &str = "crates_io.db_dump_upload_duration_ns";

/// The error type attribute key.
pub const ERROR_TYPE: &str = "error.type";

/// The archive format attribute key.
pub const FORMAT: &str = "format";

/// The HTTP request method attribute key.
pub const HTTP_REQUEST_METHOD: &str = "http.request.method";

/// The matched HTTP route attribute key.
pub const HTTP_ROUTE: &str = "http.route";

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
