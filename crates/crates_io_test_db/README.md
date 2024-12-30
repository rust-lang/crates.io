# crates_io_test_db

This package contains the code necessary to create test databases for testing
purposes.

`TestDatabase::new()` can be used to create a new test database, based on a
template database, which is lazily created the first time it is needed.

The databases are created based on the `TEST_DATABASE_URL` environment variable,
which should be set to a valid database URL. The template database will then be
created with a similar name and `_template` suffix, while the test databases
will use random suffixes.

Note that the template database will be created with applied database migrations,
so if you need an empty database, this is not the right tool for you.
