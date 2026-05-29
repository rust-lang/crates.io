# Migrations

This directory contains the database migrations for crates.io, managed by [Diesel](https://diesel.rs). This README covers how migrations are applied in production, the local development workflow, and the constraints to keep in mind so that a migration deploys safely.

## How migrations run in production

Migrations are applied during the Heroku release phase, which runs `crates-admin migrate` (see [`Procfile`](../Procfile)) before any new code goes live. The release phase runs while the old dynos are still serving requests, and only after it succeeds are those dynos replaced with new ones running the new code.

This leaves a window in which the old codebase talks to an already-migrated database, so a migration has to work against both the old and the new code.

If the release phase fails, Heroku aborts the deploy and the old dynos keep running, so a broken migration blocks the release rather than taking the site down. When the database is in read-only mode, for example during maintenance or while mitigating an outage, migrations are skipped so that configuration changes can still be deployed.

## Local workflow

During development you work against your local database with the Diesel CLI:

```bash
diesel migration generate <name>  # Create a new migration
diesel migration run              # Apply pending migrations
diesel migration revert           # Revert the most recent migration
diesel migration redo             # Revert and reapply the most recent migration
```

`generate` creates a directory with an `up.sql` and a `down.sql`. Put the schema change in `up.sql` and the statements that undo it in `down.sql`. After applying it with `diesel migration run`, use `diesel migration redo`, which reverts and reapplies the migration, to check that `down.sql` works and that `up.sql` can be applied again cleanly.

## Writing backward-compatible migrations

As described above, a migration runs while the old code is still serving traffic, so it cannot make a change that the running code can't tolerate. Dropping or renaming a column or table that the old code still reads, or adding a `NOT NULL` column the old code doesn't populate, will break those requests during the release window.

The way around this is to split a breaking change across several deploys, often called the expand/contract pattern. First expand the schema so it supports both the old and the new code, for example by adding the new column alongside the old one. Then move the application code over to the new shape and backfill any data. Once nothing references the old column anymore, a later migration can contract by removing it.

## diesel-guard

[diesel-guard](https://crates.io/crates/diesel-guard) checks migrations for patterns that are unsafe to apply against a running database, and it runs in CI on every change to this directory. You can run the same check locally before pushing:

```bash
diesel-guard check migrations/
```

Its configuration lives in [`diesel-guard.toml`](../diesel-guard.toml). The `start_after` setting tells it to ignore migrations created before a given date. New versions of diesel-guard often add new rules, and `start_after` keeps those rules from retroactively flagging migrations that have already been applied in production.
