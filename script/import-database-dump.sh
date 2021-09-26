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

cd $DUMP_PATH
echo "Creating 'cargo_registry' database"
psql --command="DROP DATABASE IF EXISTS cargo_registry"
psql --command="CREATE DATABASE cargo_registry"

echo "Importing database schema"
psql -a cargo_registry < schema.sql

echo "Importing data"
psql -a cargo_registry < import.sql
