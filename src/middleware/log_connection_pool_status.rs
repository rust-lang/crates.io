//! Log the current state of the database connection pool at most once per minute

use super::prelude::*;
use crate::app::App;

use conduit::Request;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex, PoisonError,
};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub(crate) struct LogConnectionPoolStatus {
    app: Arc<App>,
    last_log_time: Arc<Mutex<Instant>>,
    in_flight_requests: Arc<AtomicUsize>,
}

impl LogConnectionPoolStatus {
    pub(crate) fn new(app: &Arc<App>) -> Self {
        Self {
            app: app.clone(),
            last_log_time: Arc::new(Mutex::new(Instant::now())),
            in_flight_requests: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl Middleware for LogConnectionPoolStatus {
    fn before(&self, _: &mut dyn Request) -> Result<(), Box<dyn Error + Send>> {
        let mut last_log_time = self
            .last_log_time
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let in_flight_requests = self.in_flight_requests.fetch_add(1, Ordering::SeqCst);
        if last_log_time.elapsed() >= Duration::from_secs(60) {
            *last_log_time = Instant::now();
            println!(
                "connection_pool_status=\"{:?}\" in_flight_requests={}",
                self.app.diesel_database.state(),
                in_flight_requests
            );
        }
        Ok(())
    }

    fn after(
        &self,
        _: &mut dyn Request,
        res: Result<Response, Box<dyn Error + Send>>,
    ) -> Result<Response, Box<dyn Error + Send>> {
        self.in_flight_requests.fetch_sub(1, Ordering::SeqCst);
        res
    }
}
