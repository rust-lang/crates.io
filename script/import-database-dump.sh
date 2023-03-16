#!/bin/sh

# Downloads the database dump tarball from crates.io and imports it
# into the `cargo_registry` database. If the database already exists it
# is recreated from scratch!

set -o errexit
set -o nounset
set -o pipefail

readonly TARBALL_PATH="tmp/db-dump.tar.gz"
readonly DUMP_PATH="tmp/db-dump"

if [ -f "$TARBALL_PATH" ]; then
    echo "Skipping https://static.crates.io/db-dump.tar.gz download since it exists already "
else
    echo "Downloading https://static.crates.io/db-dump.tar.gz to the 'tmp' folder"
    curl https://static.crates.io/db-dump.tar.gz --output $TARBALL_PATH
fi

if [ -d "$DUMP_PATH" ]; then
    echo "Skipping db-dump.tar.gz extraction since '$DUMP_PATH' exists already"
else
    echo "Extracting db-dump.tar.gz to '$DUMP_PATH'"
    mkdir -p $DUMP_PATH
    tar -xf $TARBALL_PATH --strip 1 -C $DUMP_PATH
fi

# Figure out which database to connect to, using the psql standard $PGDATABASE
# first, otherwise extracting it from $DATABASE_URL as defined in .env. If
# that's unset, then we'll fall back to the hard-coded default cargo_registry.
if [ -n "${PGDATABASE+x}" ]; then
  DATABASE_NAME="$PGDATABASE"
elif [ -n "${DATABASE_URL+x}" ]; then
  DATABASE_NAME="$(echo "$DATABASE_URL" | awk -F / '{ print $NF }')"
else
  DATABASE_NAME=cargo_registry
fi
readonly DATABASE_NAME

# PostgreSQL doesn't permit dropping a database with active connections, so we
# need to connect to another database. While `postgres` is technically not
# required to be present, in practice it almost always is, including if the
# standard `postgres` container is being used in Docker.
readonly DROP_CREATE_DATABASE_NAME="${DROP_CREATE_DATABASE_NAME:-postgres}"

cd $DUMP_PATH
echo "Creating '$DATABASE_NAME' database"
psql --command="DROP DATABASE IF EXISTS $DATABASE_NAME" "$DROP_CREATE_DATABASE_NAME"
psql --command="CREATE DATABASE $DATABASE_NAME" "$DROP_CREATE_DATABASE_NAME"

echo "Importing database schema"
psql -a "$DATABASE_NAME" < schema.sql

echo "Importing data"
psql -a "$DATABASE_NAME" < import.sql

# Importing the database doesn't cause materialised views to be refreshed, so
# let's do that.
psql --command="REFRESH MATERIALIZED VIEW recent_crate_downloads" "$DATABASE_NAME"
