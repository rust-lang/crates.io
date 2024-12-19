# `crates_io_diesel_helpers`

This package contains a set of helper functions for working with the
[`diesel`](https://crates.io/crates/diesel) database crate.

Specifically, it contains:

- various `define_sql_function!()` calls to define SQL functions to be used in
  Diesel queries (e.g. `lower()`, `floor()`, etc.)

- a deserialization helper for `semver::Version`

- a `pg_enum!()` macro to define an enum based on a PostgreSQL integer column

This package was extracted from the main application to avoid repeated 
recompilation of these macros when the main application is recompiled.
