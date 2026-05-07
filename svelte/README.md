# crates.io frontend

SvelteKit application that powers the [crates.io](https://crates.io) web UI. Pages consume the JSON API exposed by the Rust backend in `/src/`.

## Quick start

Run `pnpm install` from the repo root, then run `pnpm dev:live` from this folder to start the dev server against the production backend. See [`docs/CONTRIBUTING.md`](../docs/CONTRIBUTING.md#building-and-serving-the-frontend) for other backends and full setup.

## Layout

- `src/routes/` - page routes and data loaders
- `src/lib/` - shared components, stores, and utilities
- `static/` - static assets served at `/`

## Testing

- **Vitest** runs unit and component tests colocated with the code under `src/`.
- **Playwright** runs full-app browser tests in [`/e2e/`](../e2e/). The backend is mocked by [`packages/crates-io-msw/`](../packages/crates-io-msw/) instead of using a live Rust backend. [Percy](https://percy.io/) captures page snapshots from this suite on every PR for visual regression testing.
- **Storybook and Chromatic** handle component development and visual regression testing. Stories (`*.stories.svelte`) live alongside their components, and `pnpm storybook` from this folder opens them locally. [Chromatic](https://www.chromatic.com/) renders and checks the same stories on every PR.

See [`docs/CONTRIBUTING.md`](../docs/CONTRIBUTING.md#running-the-frontend-tests) for commands.

## Related documentation

- [`docs/CONTRIBUTING.md`](../docs/CONTRIBUTING.md#working-on-the-frontend) - setup, dev commands, testing
- [`docs/ARCHITECTURE.md`](../docs/ARCHITECTURE.md) - system design
- [`AGENTS.md`](../AGENTS.md) - command summary for AI agents
