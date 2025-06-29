# crates_io_worker

A robust background job processing system for the crates.io application.

## Overview

This crate provides an async PostgreSQL-backed job queue system with support for:

- **Prioritized job execution** with configurable priorities
- **Job deduplication** to prevent duplicate work
- **Multiple job queues** with independent worker pools
- **Automatic retry** with exponential backoff for failed jobs
- **Graceful shutdown** and queue management
- **Error tracking** with Sentry integration

## Architecture

The system consists of three main components:

- **`BackgroundJob`** trait - Define job types and their execution logic
- **`Runner`** - High-level orchestrator that manages multiple queues and their worker pools
- **`Worker`** - Low-level executor that polls for and processes individual jobs

### Runner vs Worker

- **`Runner`** is the entry point and orchestrator:
  - Manages multiple named queues (e.g., "default", "emails", "indexing")
  - Spawns and coordinates multiple `Worker` instances per queue
  - Handles job type registration and queue configuration
  - Provides graceful shutdown coordination across all workers

- **`Worker`** is the actual job processor:
  - Polls the database for available jobs in a specific queue
  - Locks individual jobs to prevent concurrent execution
  - Executes job logic with error handling and retry logic
  - Reports job completion or failure back to the database

Jobs are stored in the `background_jobs` PostgreSQL table and processed asynchronously by worker instances that poll for available work in their assigned queues.

### Job Processing and Locking

When a worker picks up a job from the database, the table row is immediately locked to prevent other workers from processing the same job concurrently. This ensures that:

- Each job is processed exactly once, even with multiple workers running
- Failed jobs can be safely retried without duplication
- The system scales horizontally by adding more worker processes

Once job execution completes successfully, the row is deleted from the table. If the job fails, the row remains with updated retry information for future processing attempts.

## Database Schema

```sql
CREATE TABLE background_jobs (
    id BIGSERIAL PRIMARY KEY,
    job_type TEXT NOT NULL,
    data JSONB NOT NULL,
    retries INTEGER NOT NULL DEFAULT 0,
    last_retry TIMESTAMP NOT NULL DEFAULT NOW(),
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    priority SMALLINT NOT NULL DEFAULT 0
);
```

## Usage

### Defining a Job

```rust, ignore
use crates_io_worker::BackgroundJob;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SendEmailJob {
    to: String,
    subject: String,
    body: String,
}

impl BackgroundJob for SendEmailJob {
    const JOB_NAME: &'static str = "send_email";
    const PRIORITY: i16 = 10;
    const DEDUPLICATED: bool = false;
    const QUEUE: &'static str = "emails";

    type Context = AppContext;

    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        // Job implementation
        ctx.email_service.send(&self.to, &self.subject, &self.body).await?;
        Ok(())
    }
}
```

### Running the Worker

```rust,ignore
use crates_io_worker::Runner;
use std::time::Duration;

let runner = Runner::new(connection_pool, app_context)
    .register_job_type::<SendEmailJob>()
    .configure_queue("emails", |queue| {
        queue.num_workers(2).poll_interval(Duration::from_secs(5))
    });

let handle = runner.start();
handle.wait_for_shutdown().await;
```

### Enqueuing Jobs

```rust,ignore
let job = SendEmailJob {
    to: "user@example.com".to_string(),
    subject: "Welcome!".to_string(),
    body: "Thanks for signing up!".to_string(),
};

job.enqueue(&mut conn).await?;
```

## Configuration

### Job Properties

- **`JOB_NAME`**: Unique identifier for the job type
- **`PRIORITY`**: Execution priority (higher values = higher priority)
- **`DEDUPLICATED`**: Whether to prevent duplicate jobs with identical data
- **`QUEUE`**: Queue name for job execution (defaults to "default")

### Queue Configuration

- **Worker count**: Number of concurrent workers per queue
- **Poll interval**: How often workers check for new jobs
- **Shutdown behavior**: Whether to stop when queue is empty

## Error Handling

Failed jobs are automatically retried with exponential backoff. The retry count and last retry timestamp are tracked in the database. Jobs that continue to fail will eventually be abandoned after reaching the maximum retry limit.

All job execution is instrumented with tracing and optionally reported to Sentry for error monitoring.

## History

The implementation was originally extracted from crates.io into the separate
[`swirl`](https://github.com/sgrif/swirl) project, but has since been
re-integrated and heavily modified to meet the specific needs of the crates.io platform.
