use crate::background_job::DEFAULT_QUEUE;
use crate::job_registry::JobRegistry;
use crate::worker::Worker;
use crate::{storage, BackgroundJob};
use anyhow::anyhow;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PoolError, PooledConnection};
use futures_util::future::join_all;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use tracing::{info, info_span, warn, Instrument};

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(1);

pub type ConnectionPool = Pool<ConnectionManager<PgConnection>>;
pub type PooledConn = PooledConnection<ConnectionManager<PgConnection>>;

/// The core runner responsible for locking and running jobs
pub struct Runner<Context> {
    rt_handle: Handle,
    connection_pool: ConnectionPool,
    queues: HashMap<String, Queue<Context>>,
    context: Context,
    shutdown_when_queue_empty: bool,
}

impl<Context: Clone + Send + Sync + 'static> Runner<Context> {
    pub fn new(rt_handle: &Handle, connection_pool: ConnectionPool, context: Context) -> Self {
        Self {
            rt_handle: rt_handle.clone(),
            connection_pool,
            queues: HashMap::new(),
            context,
            shutdown_when_queue_empty: false,
        }
    }

    /// Register a new job type for this job runner.
    pub fn register_job_type<J: BackgroundJob<Context = Context>>(mut self) -> Self {
        let queue = self.queues.entry(J::QUEUE.into()).or_default();
        queue.job_registry.register::<J>();
        self
    }

    /// Adjust the configuration of the [DEFAULT_QUEUE] queue.
    pub fn configure_default_queue<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut Queue<Context>) -> &Queue<Context>,
    {
        self.configure_queue(DEFAULT_QUEUE, f)
    }

    /// Adjust the configuration of a queue. If the queue does not exist,
    /// it will be created.
    pub fn configure_queue<F>(mut self, name: &str, f: F) -> Self
    where
        F: FnOnce(&mut Queue<Context>) -> &Queue<Context>,
    {
        f(self.queues.entry(name.into()).or_default());
        self
    }

    /// Set the runner to shut down when the background job queue is empty.
    pub fn shutdown_when_queue_empty(mut self) -> Self {
        self.shutdown_when_queue_empty = true;
        self
    }

    /// Start the background workers.
    ///
    /// This returns a `RunningRunner` which can be used to wait for the workers to shutdown.
    pub fn start(&self) -> RunHandle {
        let mut handles = Vec::new();
        for (queue_name, queue) in &self.queues {
            for i in 1..=queue.num_workers {
                let name = format!("background-worker-{queue_name}-{i}");
                info!(worker.name = %name, "Starting worker…");

                let worker = Worker {
                    connection_pool: self.connection_pool.clone(),
                    context: self.context.clone(),
                    job_registry: Arc::new(queue.job_registry.clone()),
                    shutdown_when_queue_empty: self.shutdown_when_queue_empty,
                    poll_interval: queue.poll_interval,
                };

                let span = info_span!("worker", worker.name = %name);
                let handle = self
                    .rt_handle
                    .spawn(async move { worker.run().instrument(span).await });

                handles.push(handle);
            }
        }

        RunHandle { handles }
    }

    pub fn connection(&self) -> Result<PooledConn, PoolError> {
        self.connection_pool.get()
    }

    /// Check if any jobs in the queue have failed.
    ///
    /// This function is intended for use in tests and will return an error if
    /// any jobs have failed.
    pub fn check_for_failed_jobs(&self) -> anyhow::Result<()> {
        let failed_jobs = storage::failed_job_count(&mut *self.connection()?)?;
        if failed_jobs == 0 {
            Ok(())
        } else {
            Err(anyhow!("{failed_jobs} jobs failed"))
        }
    }
}

pub struct RunHandle {
    handles: Vec<JoinHandle<()>>,
}

impl RunHandle {
    /// Wait for all background workers to shut down.
    pub async fn wait_for_shutdown(self) {
        join_all(self.handles).await.into_iter().for_each(|result| {
            if let Err(error) = result {
                warn!(%error, "Background worker task panicked");
            }
        });
    }
}

pub struct Queue<Context> {
    job_registry: JobRegistry<Context>,
    num_workers: usize,
    poll_interval: Duration,
}

impl<Context> Default for Queue<Context> {
    fn default() -> Self {
        Self {
            job_registry: JobRegistry::default(),
            num_workers: 1,
            poll_interval: DEFAULT_POLL_INTERVAL,
        }
    }
}

impl<Context> Queue<Context> {
    /// Set the number of workers to spawn for this queue.
    pub fn num_workers(&mut self, num_workers: usize) -> &mut Self {
        self.num_workers = num_workers;
        self
    }

    /// Set the interval after which each worker of this queue polls for new jobs.
    pub fn poll_interval(&mut self, poll_interval: Duration) -> &mut Self {
        self.poll_interval = poll_interval;
        self
    }
}
