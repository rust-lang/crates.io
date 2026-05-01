# E2E Overview

End-to-end tests for the SvelteKit frontend, written with [Playwright](https://playwright.dev/).

The tests live alongside their fixtures and helpers in this directory and are wired to the
Svelte preview server via `playwright.config.ts` at the repo root. Run them with `pnpm e2e:svelte`
from the repo root, or `pnpm e2e:svelte <spec_file>` to run a single spec.

The MSW handlers in `/packages/crates-io-msw/` provide a mocked backend for these tests, so they
do not need a running crates.io backend.
