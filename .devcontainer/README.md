# Devcontainer

A single full-stack devcontainer that provides everything needed to work on
crates.io: backend, frontend, and a Postgres database. This is the
recommended setup for new contributors.

## What's included

- Rust toolchain (managed by `rustup`, pinned via `rust-toolchain.toml`)
- Node.js and pnpm
- Postgres, accessible at the hostname `db`
- `psql` client and `libpq-dev`
- `diesel_cli` (with the `postgres` feature)
- Playwright Chromium headless shell with the required system libraries

## How to use it

### VS Code

Open the repository in VS Code with the [Dev Containers] extension installed
and select "Reopen in Container".

[Dev Containers]: https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers

### GitHub Codespaces

Open the repository on GitHub and create a new codespace. The container
starts automatically.

### Command line

With the [`devcontainer` CLI] installed:

```console
devcontainer up
devcontainer exec bash
```

[`devcontainer` CLI]: https://github.com/devcontainers/cli

## What happens on first start

The `postCreateCommand` runs once after the container is built and:

1. Initializes the local git index in `tmp/index-bare/`
2. Creates the `cargo_registry_test` database
3. Runs Diesel migrations against `cargo_registry`
4. Installs JavaScript dependencies with `pnpm install`
5. Installs the Playwright Chromium headless shell
6. Pre-fetches Rust dependencies with `cargo fetch`

On every subsequent container start, `diesel migration run` is invoked to
keep the development database schema current.

## Running the dev servers

The container only provisions tooling and dependencies; it does not
start the backend or the Svelte dev server. See [Building and serving
the frontend](../docs/CONTRIBUTING.md#building-and-serving-the-frontend)
and [Starting the server and the frontend](../docs/CONTRIBUTING.md#starting-the-server-and-the-frontend)
in `CONTRIBUTING.md` for the commands.

## Database

The `cargo_registry` database is empty by default. To populate it with a
recent snapshot of the production data, run:

```console
script/import-database-dump.sh
```

The dump is several GB compressed and the import takes a while.

## GitHub OAuth credentials

The container's first start copies `.env.sample` to `.env` if one
doesn't already exist. The defaults are enough for the backend to
boot. Login flows additionally require a GitHub OAuth app, so fill
in the `GH_CLIENT_ID` and `GH_CLIENT_SECRET` values. `.env.sample`
documents the callback URL to register with GitHub.

## Forwarded ports

| Port | Service          |
|------|------------------|
| 8888 | Backend API      |
| 5173 | Svelte dev server|
| 6006 | Storybook        |

## Persistent volumes

Four named volumes persist between container rebuilds:

- `/workspaces/crates.io/target` (Cargo build output)
- `/usr/local/cargo/registry` (downloaded crate sources)
- `/home/vscode/.local/share/pnpm/store` (pnpm content-addressable store)
- `/workspaces/crates.io/local_uploads` (files written by the backend
  when storage isn't configured for S3, e.g. when publishing crates to
  your local instance)
