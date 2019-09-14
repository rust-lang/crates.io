# crates.io Database Dump

This is a dump of the public information in the crates.io database.

## Files

* `data/` – the CSV files with the actual dump data.
* `export.sql` – the `psql` script that was used to create this database dump. It is only included in the archive for reference.
* `import.sql` – a `psql` script that can be used to restore the dump into a PostgreSQL database with the same schema as the `crates.io` database, destroying all current data.
* `metadata.json` – some metadata of this dump.

## Metadata Fields

* `timestamp` – the UTC time the dump was started.
* `crates_io_commit` – the git commit hash of the deployed version of crates.io that created this dump.
* `format_version` – the version of the layout and format of this dump. Roughly follows SemVer conventions.

## Restoring to a Local crates.io Database

1. Create a new database.

        createdb DATABASE_NAME

2. Restore the database schema.

        psql DATABASE_NAME < schema.sql

3. Run the import script.

        psql DATABASE_URL < import.sql
