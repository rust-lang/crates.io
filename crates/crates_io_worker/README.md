# crates_io_worker

This package contains the background job runner for the crates.io application.

The implementation was originally extracted from crates.io into the separate
[`swirl`](https://github.com/sgrif/swirl) project, but has since been
re-integrated and heavily modified.

The background worker uses a `background_jobs` PostgreSQL table to store jobs
that need to be run. Once a job is picked up by a worker, the table row is
locked, and the job is run. If the job fails, it will be retried with
exponential backoff. If the job succeeds, the row will be deleted.
