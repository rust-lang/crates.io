# crates.io

## Repository Layout

- `/src/` - Backend Rust application code
  - `/src/bin/` - Binary entry points: `server.rs` (main API server), `background-worker.rs` (async job processor), `monitor.rs` (monitoring tool), `crates-admin/` (admin CLI tools)
  - `/src/controllers/` - API controllers organized by domain (`krate/`, `user/`, `version/`, `trustpub/`, `github/`, `admin/`)
  - `/src/worker/jobs/` - Background job implementations (crate analysis, README rendering, OG image generation, database dumps)
  - `/src/middleware/` - Request processing middleware (auth, rate limiting, logging, error handling)
  - `/src/tests/` - Backend integration tests with snapshot testing using `insta`
  - `/src/config/` - Configuration loading and validation
  - `/src/util/` - Shared utilities (errors, authentication, pagination)
- `/svelte/` - Frontend SvelteKit application
  - `/svelte/src/routes/` - File-based routes and page components
  - `/svelte/src/lib/` - Shared components, stores, services, and utilities
  - `/svelte/static/` - Static assets served at `/` (favicon, robots.txt, etc.)
- `/crates/` - Workspace crates providing specialized functionality
  - `crates_io_api_types/` - API response serialization types
  - `crates_io_database/` - Database models and schema (Diesel ORM)
  - `crates_io_worker/` - Background job queue system
  - `crates_io_index/` - Git index management for crate metadata
  - `crates_io_tarball/` - Package tarball processing and validation
  - `crates_io_trustpub/` - Trusted Publishing implementation
  - `crates_io_markdown/`, `crates_io_linecount/` - Content processing
- `/migrations/` - Database migrations (260+ historical migrations managed by Diesel)
- `/e2e/` - Playwright tests for the frontend with accessibility checks
- `/packages/` - MSW (Mock Service Worker) test utilities for API mocking
- `/script/` - Development utilities
- `/docs/` - Project documentation (`CONTRIBUTING.md`, `ARCHITECTURE.md`, PR review guidelines)

## General Guidance

- Read `/docs/CONTRIBUTING.md` for comprehensive setup instructions and contribution guidelines.
- Use `cargo insta accept` instead of `cargo insta review` when updating snapshot tests.
- Use `#[expect(...)]` instead of `#[allow(...)]` to silence warnings that should be resolved later.
- Match existing code style within each area (backend Rust follows workspace lints, frontend follows ESLint/Prettier config).
- Add documentation comments to new types and functions; existing code may lack comments, but new code should have them.

## Backend

### Building and Running

Run the API server:

```bash
cargo run
```

Run the background worker:

```bash
cargo run --bin background-worker
```

Database migrations:

```bash
diesel migration run              # Apply pending migrations
diesel migration revert           # Revert last migration
diesel migration redo             # Revert and reapply last migration
diesel migration generate <name>  # Create new migration
```

Admin CLI (check crate ownership, manage users, etc.):

```bash
cargo run --bin crates-admin -- <subcommand>
```

### Testing

Run backend tests (with separate test database):

```bash
cargo test
```

Run specific tests:

```bash
cargo test <test_name>
```

Accept updated snapshot tests:

```bash
cargo insta accept --workspace
```

Check code quality:

```bash
cargo fmt --all --check                                # Formatting
cargo clippy --all-targets --all-features --workspace  # Linting
```

Test database setup: Set `TEST_DATABASE_URL` in `.env` to a separate database (e.g., `postgres://postgres@localhost/cargo_registry_test`). The test harness creates isolated databases and runs migrations automatically. Create the base test database once with `createdb cargo_registry_test`.

### Architecture and Conventions

- Use Axum web framework patterns (handlers, extractors, middleware).
- Follow Diesel ORM patterns for database queries; see models in `crates/crates_io_database/src/models/`.
- Use `anyhow::Result` for fallible operations; use `thiserror` for domain-specific error types.
- Background jobs go in `/src/worker/jobs/` and must implement the `Job` trait from `crates_io_worker`. Jobs must be idempotent.
- Controllers return responses via helper functions in `/src/util/` for consistent serialization.
- Use snapshot testing with `insta` for API responses.
- Never ignore deprecation warnings; fix them immediately or use `#[expect(deprecated)]`.
- Rate limiting configuration lives in `/src/rate_limiter.rs`; new endpoints should use appropriate limiters.
- Structured logging uses `tracing`; add spans for request context.
- Authentication happens via session cookies (web users) or API tokens (cargo CLI, third-party clients).
- OpenAPI spec: Auto-generated from backend code; update by running `cargo test --package crates_io --lib openapi` and accepting the snapshot changes.
- New API endpoints should follow the conventions in [`docs/API-DESIGN.md`](docs/API-DESIGN.md) (URL shape, request/response envelopes, errors, pagination, auth, OpenAPI annotations).

## Frontend

The frontend is a SvelteKit application living in `/svelte/`. Most frontend
commands run from inside that directory or via `pnpm --filter
crates.io-svelte <script>` from the repo root.

### Building and Running

Install dependencies (from the repo root):

```bash
pnpm install
pnpm playwright install chromium-headless-shell  # Install necessary Playwright browsers
```

Development server options (from `svelte/`):

```bash
pnpm dev:live      # Use production crates.io backend
pnpm dev:staging   # Use staging backend
pnpm dev:local     # Use local backend (requires backend setup)
pnpm dev:msw       # Use MSW handlers from /packages/crates-io-msw/ as the backend
```

The dev server listens on <http://localhost:5173>.

Build for production (from `svelte/`):

```bash
pnpm build
```

### Testing

Run tests:

```bash
pnpm --filter crates.io-svelte test        # Vitest unit/component tests
pnpm --filter "@crates-io/msw" test        # MSW package tests
pnpm e2e:svelte                            # Playwright end-to-end tests (from repo root)
pnpm e2e:svelte <test_file>                # Run a specific Playwright spec
```

Code quality (from the repo root unless noted):

```bash
pnpm lint:js                            # Run ESLint
pnpm prettier:check                     # Check formatting
pnpm prettier:write                     # Fix formatting
```

### Architecture and Conventions

- Use Svelte 5 runes (`$state`, `$derived`, `$effect`) for reactivity; avoid the legacy `<script>` reactivity patterns.
- Routes live under `/svelte/src/routes/` and follow SvelteKit's file-based conventions; data loading happens in `+page.ts` / `+page.server.ts` `load` functions.
- Shared state goes in `/svelte/src/lib/` (stores, services); reusable UI in `/svelte/src/lib/components/`.
- Components are styled with scoped `<style>` blocks in `.svelte` files.
- Accessibility is critical; use semantic HTML, ARIA attributes, and verify with the e2e a11y fixtures.
- MSW mocks for tests live in `/packages/crates-io-msw/`; define handlers for API endpoints.
- Visual regression testing uses Percy (via Playwright) and Chromatic via Storybook
- Follow existing component patterns for consistency; search for similar components before creating new ones.

## Commit Messages and Pull Requests

Use present tense imperative mood ("Add", "Fix", "Remove", "Use", "Implement", "Extract"). Include scope prefixes when relevant, for example:

- `trustpub:` for trusted publishing changes
- `jobs/<job_name>:` for background jobs
- `controllers/<area>:` for controller changes

Use backticks for code elements: `function_name()`, `StructName`, `ModuleName`.

Examples:

- "Use `#[expect(deprecated)]` to silence `generic-array` deprecations"
- "Fix dropdown indicator color in dark mode"
- "trustpub: Derive `Clone` for claims structs"
- "jobs/analyze_crate_file: Schedule OG image rerender after analysis"
- "Extract `validate_tarball()` fn"
- "Implement `Display` for `ErrorKind`"

Keep first line under 72 characters. Be direct and technical; omit unnecessary articles. For complex changes, add a commit body explaining the reasoning.

Pull requests run CI checks: backend tests, Svelte and MSW unit tests, Playwright e2e tests, ESLint, Prettier, rustfmt, and clippy. Fix issues before merging. The `main` branch auto-deploys to staging; production deployments are manual promotions.

## Review Checklist

Before submitting:

- Run `cargo fmt --all` and `pnpm prettier:write` from the repo root for consistent formatting.
- Run `cargo clippy` and fix warnings.
- Run the relevant test suites for the changed files; all must pass:
  - Backend: `cargo test`
  - Svelte unit/component tests: `pnpm --filter crates.io-svelte test`
  - MSW package tests: `pnpm --filter "@crates-io/msw" test`
  - Playwright: `pnpm e2e:svelte`
- Accept snapshot changes with `cargo insta accept` if expected.
- Check that new backend functions/types have documentation comments.
- Verify frontend changes work against production data with `pnpm dev:live` (from `svelte/`).
- Ensure database migrations are reversible (test with `diesel migration redo`).
- Confirm error messages are actionable and don't expose sensitive information.
- Test accessibility with keyboard navigation and screen reader landmarks.

## Reference Material

- `/docs/CONTRIBUTING.md` - Setup instructions, detailed workflows, Docker setup, GitHub OAuth configuration
- `/docs/ARCHITECTURE.md` - System architecture and design decisions
- `/docs/API-DESIGN.md` - JSON API conventions for new endpoints
- `/docs/PR-REVIEW.md` - Guidelines for reviewing pull requests
- `/script/import-database-dump.sh` - Import production database dump for testing
- `/.github/workflows/ci.yml` - CI pipeline definition and tool versions
