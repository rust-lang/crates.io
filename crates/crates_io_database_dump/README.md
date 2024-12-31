# crates_io_database_dump

This package contains the code and data to create a database dump for the
crates.io database.

The most important file in this package is the `dump-db.toml` file, which
defines how the database tables are serialized into CSV files. Specifically,
it can be used to skip certain columns for privacy reasons, it can declare the
serialization order of the tables, and it can declare filters, if not all rows
should be dumped.
