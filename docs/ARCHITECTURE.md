# Architecture of crates.io

This document is an intro to the codebase in this repo. If you want to work on a bug or a feature,
hopefully after reading this doc, you'll have a good idea of where to start looking for the code
you want to change.

This is a work in progress. Pull requests and issues to improve this document are very welcome!

## Documentation

Documentation about the codebase appears in these locations:

- `LICENSE-APACHE` and `LICENSE-MIT` - the terms under which this codebase is licensed.
- `README.md` - Important information we want to show on the github front page.
- `docs/` - Long-form documentation.

## Backend - Rust

The backend of crates.io is written in Rust. Most of that code lives in the _src_ directory. It
serves a JSON API over HTTP, and the HTTP server interface is provided by the [axum][] crate and
related crates. More information about backend development is in
[`docs/CONTRIBUTING.md`](https://github.com/rust-lang/crates.io/blob/master/docs/CONTRIBUTING.md).

[axum]: https://crates.io/crates/axum

These files and directories have to do with the backend:

- `Cargo.lock` - Locks dependencies to specific versions providing consistency across development
  and deployment
- `Cargo.toml` - Defines the crate and its dependencies
- `migrations/` - Diesel migrations applied to the database during development and deployment
- `src/` - The backend's source code
- `target/` - Compiled output, including dependencies and final binary artifacts - (ignored in
  `.gitignore`)

The backend stores information in a Postgres database.

## Frontend - SvelteKit

The frontend of crates.io is a [SvelteKit][] application written in TypeScript, served as a
static site via `@sveltejs/adapter-static`. More information about frontend development is in
[`docs/CONTRIBUTING.md`](https://github.com/rust-lang/crates.io/blob/master/docs/CONTRIBUTING.md).

[SvelteKit]: https://svelte.dev/docs/kit/introduction

These files have to do with the frontend:

- `svelte/` - The frontend application (its own pnpm workspace package)
  - `svelte/src/routes/` - File-based routes; pages, layouts, and `+server.ts` endpoints
  - `svelte/src/lib/` - Shared components, runes-based stores, services, and utilities
  - `svelte/static/` - Static assets served at `/`
  - `svelte/svelte.config.js`, `svelte/vite.config.ts` - Build and dev-server configuration
  - `svelte/build/` - Output of `pnpm build`; served under the root `/` url - (ignored in
    `.gitignore`)
- `e2e/` - Playwright end-to-end tests run against the Svelte app
- `eslint.config.mjs` - JavaScript/TypeScript coding style
- `node_modules/` - npm dependencies - (ignored in `.gitignore`)
- `packages/crates-io-msw/` - A mock backend used for testing
- `package.json` - Root npm package; defines workspace-wide scripts and dev tooling
- `pnpm-lock.yaml` - Locks dependencies to specific versions providing consistency across
  development and deployment
- `pnpm-workspace.yaml` - pnpm workspace declaration (`packages/*`, `svelte/`)

## Deployment - Heroku

crates.io is deployed on [Heroku](https://heroku.com/).

These files are Heroku-specific; if you're deploying the crates.io codebase on another platform,
there's useful information in these files that you might need to translate to a different format
for another platform.

- `.buildpacks` - A list of buildpacks used during deployment
- `config/nginx.conf.erb` - Template used by the nginx buildpack
- `.diesel_version` - Used by diesel buildpack to install a specific version of Diesel CLI during
  deployment
- `Procfile` - Contains process type declarations for Heroku

## Development

These files are mostly only relevant when running crates.io's code in development mode.

- `.editorconfig` - Coding style definitions supported by some IDEs
  - [EditorConfig for VS Code]
  - [EditorConfig for JetBrains IDEs]
  - More plugins are available at: https://editorconfig.org/#download
- `.env` - Environment variables loaded by the backend - (ignored in `.gitignore`)
- `.env.sample` - Example environment file checked into the repository
- `.git/` - The git repository; not available in all deployments (e.g. Heroku)
- `.gitignore` - Configures git to ignore certain files and folders
- `local_uploads/` - Serves crates and readmes that are published to the
  local development environment
- `script/init-local-index.sh` - Creates registry repositories used during development
- `tmp/` - Temporary files created during development; when deployed on Heroku this is the only
  writable directory - (ignored in `.gitignore`)
- `tmp/index-bare` - A bare git repository during development - (ignored in `.gitignore`)
- `.github/workflows/*` - Configuration for continuous integration at [GitHub Actions]

[GitHub Actions]: https://github.com/rust-lang/crates.io/actions
[EditorConfig for VS Code]: https://marketplace.visualstudio.com/items?itemName=EditorConfig.EditorConfig
[EditorConfig for JetBrains IDEs]: https://plugins.jetbrains.com/plugin/7294-editorconfig
