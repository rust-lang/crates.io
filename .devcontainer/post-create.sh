#!/usr/bin/env bash
set -euo pipefail

# Named volumes are mounted as root by Docker; chown so the `vscode`
# user can write to them.
sudo chown -R vscode:vscode \
    /workspaces/crates.io/target \
    /workspaces/crates.io/local_uploads \
    /usr/local/cargo/registry \
    /home/vscode/.local/share/pnpm

if [ ! -f .env ]; then
    cp .env.sample .env
fi

# `init-local-index.sh` creates a git commit; supply an identity since
# the container has none of its own.
GIT_AUTHOR_NAME=devcontainer GIT_AUTHOR_EMAIL=devcontainer@local \
GIT_COMMITTER_NAME=devcontainer GIT_COMMITTER_EMAIL=devcontainer@local \
    script/init-local-index.sh

psql -h db -U postgres <<'SQL'
SELECT 'CREATE DATABASE cargo_registry_test'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'cargo_registry_test')\gexec
SQL

diesel migration run

pnpm install

# `--with-deps` apt-installs the system libs Chromium needs. Without
# it, re-running this script fails the host-deps validator even when
# the browser binary is already cached.
pnpm playwright install --with-deps chromium-headless-shell

cargo fetch
