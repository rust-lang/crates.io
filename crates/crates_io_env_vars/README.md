# crates_io_env_vars

This package contains convenient wrappers for the `std::env::var()` function.

These functions use the `dotenvy` crate to automatically load environment
variables from a `.env` file, if it exists. This is useful for development
environments, where you don't want to set all environment variables manually.

There are also variants of the functions that make use of the `FromStr` trait to
automatically parse the environment variables into the desired types or fail
with corresponding error messages.

Finally, there are `list()` functions that allow parsing of comma-separated
lists of values.
