use tracing::dispatcher::DefaultGuard;
use tracing::subscriber;
use tracing_subscriber::fmt;

/// Enable tracing output for tests.
///
/// The tracing test output is only enabled as long as the returned guard
/// is not dropped.
pub fn enable_tracing_output() -> DefaultGuard {
    subscriber::set_default(fmt().compact().with_test_writer().finish())
}
