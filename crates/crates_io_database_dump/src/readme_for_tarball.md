# crates.io Database Dump

This is a dump of the public information in the crates.io database.

## Files

- `data/` – the CSV files with the actual data.
- `export.sql` – the `psql` script that was used to create this database dump. It is only included in the archive for reference.
- `import.sql` – a `psql` script that can be used to restore the dump into a PostgreSQL database with the same schema as the `crates.io` database, destroying all current data.
- `metadata.json` – some metadata of this dump.
- `schema.sql` – a dump of the database schema to facilitate generating a new database from the data.

## Metadata Fields

- `timestamp` – the UTC time the dump was started.
- `crates_io_commit` – the git commit hash of the deployed version of crates.io that created this dump.

## Less Obvious Database Fields

- `crate_owners.owner_kind` - if `0`, the crate owner is a user; if `1`, the crate owner is a team. (If another value, you should probably contact the crates.io team.)
- `crate_owners.owner_id` - if the owner is a user, this is their ID in `users.id`, otherwise it's the ID in `teams.id`.
- `teams.login` - this will look something like `github:foo:bar`, referring to the `bar` team in the `foo` organisation. At present, as we only support GitHub, the first component will always be `github`.

## Restoring to a Local crates.io Database

1.  Create a new database.

        createdb DATABASE_NAME

2.  Restore the database schema.

        psql DATABASE_NAME < schema.sql

3.  Run the import script.

        psql DATABASE_URL < import.sql
