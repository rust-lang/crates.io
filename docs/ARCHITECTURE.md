# Architecture of crates.io

This document describes the high-level architecture of crates.io: the major systems it is built from, how they fit together, and why. It is meant as a map both for contributors finding their way around the project and for operators reasoning about the running system. It deliberately stays high-level and avoids the details of individual modules and functions, which change far more often than the overall shape of the system. When the architecture itself changes, this document should be updated to match.

## System overview

crates.io is reached through three hostnames, all served from behind CDNs (Fastly and CloudFront). `crates.io` is the API server and web frontend, `index.crates.io` is the sparse index cargo reads crate metadata from, and `static.crates.io` serves crate files (the `.crate` files cargo downloads) along with other static assets like rendered OG images. The CDNs absorb the bulk of read traffic. Cargo fetching metadata or downloading a crate normally never reaches our backend.

Behind the CDNs, requests to `crates.io` pass through the Heroku router to a `server` dyno running our API server. The server reads from and writes to a PostgreSQL database and stores larger objects (crate files, rendered READMEs, OG images, database dumps) in Amazon S3. Work that shouldn't block a request, such as publishing to the index, invalidating CDN caches, rendering content, and counting downloads, is handed to a separate `background-worker` dyno through a Postgres-backed job queue.

## Frontend

The frontend is a [SvelteKit](https://svelte.dev/docs/kit/introduction) application written in TypeScript, living in the `svelte/` workspace. It is built as a static single-page app using `adapter-static`, with server-side rendering disabled, and the resulting assets are served by our own API server rather than from a separate host. In production the static files sit behind the CDNs like everything else on `crates.io`.

Because rendering happens entirely in the browser, the frontend talks to the backend over the same public JSON API that any other client uses. Requests go through a typed client generated from the backend's OpenAPI specification, which keeps the frontend's view of the API in sync with what the server actually serves.

## Backend processes

The backend is written in Rust and builds into a handful of separate binaries, all sharing the same codebase under `src/` and the workspace crates under `crates/`.

- **`server`** is the API server. It handles every HTTP request to `crates.io`, serves the frontend assets, and uses the [axum](https://crates.io/crates/axum) web framework. This is the only process users talk to directly.
- **`background-worker`** runs asynchronous jobs pulled from the job queue. Anything that is slow, fallible, or shouldn't block an API response happens here.
- **`monitor`** is a small process that periodically checks the health of the system and pages the on-call team through PagerDuty when something looks wrong, such as a backlog of stalled jobs, download counts that have stopped updating, or a spam attack.
- **`crates-admin`** is a command-line tool for operational tasks like deleting a crate, re-rendering READMEs, or enqueueing a job by hand. It also runs database migrations during deployment.

## PostgreSQL and migrations

PostgreSQL is the source of truth for everything except large binary objects: crates, versions, users, owners, API tokens, download counts, the background job queue, and more. The backend accesses it through [Diesel](https://diesel.rs). The `server` can be configured with a read-only replica alongside the primary, so that read-heavy traffic can be served from the replica and kept off the primary. Crate search is served directly from PostgreSQL using its full-text search rather than a separate search engine.

The database schema is managed as a series of Diesel migrations in `migrations/`. They are applied automatically during the Heroku release phase, before new code goes live, while the old dynos are still serving traffic. That overlap means every migration has to be backward-compatible with the currently-running code. In practice, breaking schema changes are split across multiple deploys using the expand/contract pattern. The [`migrations/README.md`](../migrations/README.md) covers this in detail, along with the `diesel-guard` checks that enforce it in CI.

## Object storage

Anything too large to belong in the database is kept in object storage, which is Amazon S3 in production and a local directory during development. This includes the crate files themselves, rendered README HTML, generated OG images, and the database dumps we publish for others to consume. The dumps give consumers who need bulk crate data a snapshot to download instead of scraping the API.

Crate files are immutable once published and are served to users from `static.crates.io` through the CDNs, so reads come straight from the cache and object storage rarely sees download traffic directly. The backend writes to object storage when a crate is published and when background jobs render content like READMEs and OG images.

## The crate index

For cargo to resolve dependencies it needs a list of every published version of every crate, along with their dependencies and checksums. This metadata is published separately from the database in the form of the crate index.

The index exists in two forms. The original form is a [git repository](https://github.com/rust-lang/crates.io-index) that cargo clones and pulls. The newer and now default form is the sparse index served over HTTP at `index.crates.io`, where cargo fetches only the individual files it needs instead of cloning the whole history. The sparse index files are stored in object storage and served through the CDNs.

Whenever a crate is published, yanked, or otherwise changed, a background job rewrites that crate's index entry in both forms so the two stay in sync with the database.

## CDNs

Two content delivery networks, Fastly and CloudFront, sit in front of crates.io and serve the large majority of traffic. Running two lets us fail over between providers if one has an outage. They cache responses for all three hostnames, which is what allows index reads and crate downloads to be served entirely from the edge without touching our backend. The CDN configuration itself, including rules like rewriting crate download URLs to the stored file paths, lives in the [rust-lang/simpleinfra](https://github.com/rust-lang/simpleinfra) repository rather than in this one.

Because the CDNs cache aggressively, the backend has to actively invalidate cached entries when the underlying content changes, for example when a new version updates a crate's index file. This invalidation runs as background jobs so that it doesn't slow down the request that triggered it.

## Background jobs and scheduling

Work that shouldn't happen inside a request is run asynchronously by the `background-worker` process. The job queue is stored in PostgreSQL: enqueueing a job inserts a row, and workers claim jobs by locking rows so that multiple workers never run the same job. Failed jobs are retried with an exponential backoff, so jobs are written to be idempotent and safe to run more than once. Jobs cover things like syncing the index, invalidating CDN caches, rendering READMEs and OG images, sending emails, and producing the database dumps.

Jobs are enqueued either by the API in the course of handling a request, such as a publish enqueueing an index sync, or on a schedule. Periodic jobs are triggered by the Heroku Scheduler addon, which runs the `crates-admin` CLI to enqueue them at fixed intervals.

Separately from this queue, we use an Amazon SQS queue to ingest CDN access logs, which is how downloads are counted. The download-count walkthrough below describes that flow.

## Email

crates.io sends email such as publish notifications, owner invitations, email-address confirmations, and notifications about expiring tokens. In production this goes through Mailgun over SMTP. When no SMTP server is configured, as in a local checkout, emails are instead written to a local directory, and the test suite keeps them in memory so tests can assert on what would have been sent.

## docs.rs

Documentation for published crates is built and hosted by [docs.rs](https://docs.rs), a separate service that crates.io links out to from crate pages. The two are only loosely coupled: docs.rs discovers new releases on its own by watching the crate index, so crates.io does not tell it to build anything when a crate is published. The one time crates.io reaches out to docs.rs directly is when a crate owner requests a rebuild through the site, which makes an authenticated request to the docs.rs API on their behalf.

## Authentication and authorization

Authentication depends on the kind of client. Browsers use a session cookie, created when a user logs in through GitHub, which is currently the only login mechanism. Programmatic clients like cargo and third-party tools use API tokens instead. Tokens are stored only as hashes, and they can be scoped down to particular endpoints and particular crates so that a token handed to CI can be limited to exactly what it needs.

Trusted Publishing is an alternative way for CI to obtain a token without storing a long-lived secret. Instead of configuring a durable API token, a CI workflow proves its identity with a short-lived OIDC token from a trusted provider like GitHub Actions or GitLab CI, and exchanges it for a temporary token that can only publish.

Authorization for crates is based on ownership. A crate is owned by one or more users or teams, owners can invite others, and team ownership is backed by membership in the corresponding GitHub team. Publishing, yanking, and managing owners all require the caller to be an owner of the crate.

## Observability

The backend emits structured logs through the `tracing` framework. In production these logs are shipped to DataDog, which indexes and archives them and is where we search and investigate what the running system is doing. See [`LOGGING.md`](LOGGING.md) for the conventions on how to write these logs.

Errors and panics are additionally reported to Sentry, which groups them and captures the context needed to debug them, with sensitive headers stripped before anything is sent. The backend also exposes operational metrics in Prometheus format, such as queue depths and database pool usage, for monitoring and dashboards.

On top of all this, the `monitor` process watches for specific failure conditions and pages the on-call team through PagerDuty when it finds one, so that problems like a stalled job queue get a human's attention even outside of working hours.

## Deployment

crates.io runs on Heroku. The `main` branch is automatically deployed to the staging environment at `staging.crates.io`. Production deployment is a manual process: after smoke tests pass in GitHub Actions, a team member with Heroku access can promote the staging release to production. Processes are defined in the `Procfile`: a `web` dyno (`server`), a `background_worker` dyno, and a `release` phase that runs database migrations via `crates-admin`.

## Walkthroughs

The sections above describe the systems one at a time. These walkthroughs trace a few common operations through those systems to show how they fit together in motion.

### Publishing a crate

When a user runs `cargo publish`, the following happens:

1. cargo packages the crate into a `.crate` file and uploads it to the API server at `crates.io`, authenticating with either an API token or a Trusted Publishing token.
2. The server authenticates the request, confirms the caller is allowed to publish the crate, and validates the uploaded crate file and its metadata.
3. The crate file is stored in object storage, from where it will later be served via `static.crates.io`. The new version and its metadata are recorded in PostgreSQL.
4. The server enqueues background jobs for the slower follow-up work and returns success to cargo, so the publish completes without waiting on that work.
5. The `background-worker` then rewrites the crate's entry in the index so cargo clients can see the new version, invalidates the relevant CDN caches, and renders derived content like the README and the crate's OG image.

### Downloading a crate

When cargo needs to download a crate, for example while building a project, the work is served almost entirely from the edge:

1. cargo first fetches `https://index.crates.io/config.json`, which tells it where to download crate files (the `dl` field, currently `https://static.crates.io/crates`) and the base URL of the crates.io API (the `api` field).
2. To resolve dependencies, cargo fetches the relevant per-crate files from the sparse index at `index.crates.io`. Each file lists the available versions of one crate along with their dependencies and checksums.
3. cargo then downloads the `.crate` files it needs from `{dl}/{crate}/{version}/download`. The CDN internally rewrites that download URL to the file's stored path, `{dl}/{crate}/{crate}-{version}.crate`.

cargo defaults to the sparse index described above, but the older git index still works too. Versions of cargo from before the sparse index was introduced default to the git index, and newer cargo can be pointed at it explicitly. In that mode cargo reads `config.json` and resolves dependencies from files in the git index repository instead, while crate downloads work exactly the same way.

All of this is served by the CDNs from cache, so a typical download never reaches our backend at all. This is why dependency resolution and crate downloads keep working smoothly even under heavy load or during a backend deploy.

### Counting downloads

Originally crates.io counted downloads itself. Cargo's download requests went to the API server, which recorded the download and responded with a redirect to the actual file on the CDN. This coupled downloads to the health of the API server. The server runs a limited number of worker threads, and when those were tied up handling database-backed requests, download requests queued up behind them and slowed down too, sometimes badly enough to page the on-call team.

Counting downloads from the CDNs' access logs instead is what removed that coupling. Once the API server no longer needed to see every download in order to count it, `config.json` could be changed to send cargo straight to the CDN for downloads, using the URL rewriting described above, so download traffic stopped flowing through our backend at all. The counting now happens after the fact:

1. Both CDNs are configured to deliver their access logs as files into object storage.
2. Each time a new log file is delivered, a message is placed on an Amazon SQS queue announcing it.
3. A background job consumes that queue, reads each log file, counts the crate downloads it contains, and adds them to the per-version download totals in PostgreSQL.

Because this happens in batches some time after the downloads themselves, the download numbers shown on crates.io lag slightly behind reality rather than updating live.
