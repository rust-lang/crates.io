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
related crates. More information about the backend is in
[`docs/BACKEND.md`](https://github.com/rust-lang/crates.io/blob/master/docs/BACKEND.md).

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

## Frontend - Ember.js

The frontend of crates.io is written in JavaScript using [Ember.js][]. More information about the
frontend is in [`docs/FRONTEND.md`](https://github.com/rust-lang/crates.io/blob/master/docs/FRONTEND.md).

[Ember.js]: https://emberjs.com/

These files have to do with the frontend:

- `app/` - The frontend's source code
- `config/{environment,targets}.js` - Configuration of the frontend
- `dist/` - Contains the distributable (optimized and self-contained) output of building the
  frontend; served under the root `/` url - (ignored in `.gitignore`)
- `.ember-cli` - Settings for the `ember` command line interface
- `ember-cli-build.js` - Contains the build specification for Broccoli
- `eslint.config.mjs` - Defines Javascript coding style guidelines
- `node_modules/` - npm dependencies - (ignored in `.gitignore`)
- `packages/crates-io-msw` - A mock backend used for testing
- `package.json` - Defines the npm package and its dependencies
- `package-lock.json` - Locks dependencies to specific versions providing consistency across
  development and deployment
- `public/` - Static files that are merged into `dist/` during build
- `testem.js` - Integration with Test'em Scripts
- `tests/` - Frontend tests
- `vendor/` - frontend dependencies not distributed by npm; not currently used

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
- `.watchmanconfig` - Use by Ember CLI to efficiently watch for file changes if you install watchman

[GitHub Actions]: https://github.com/rust-lang/crates.io/actions
[EditorConfig for VS Code]: https://marketplace.visualstudio.com/items?itemName=EditorConfig.EditorConfig
[EditorConfig for JetBrains IDEs]: https://plugins.jetbrains.com/plugin/7294-editorconfig
