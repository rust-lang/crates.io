# crates_io_database

This package contains the crates.io database schema as derived by `diesel print-schema`
from the database after all the migrations have been applied.

After creating new migrations (via `diesel migration generate`), you can update
the schema by running:

```sh
diesel print-schema > crates/crates_io_database/src/schema.rs
```

## `schema.patch`

Note that there is also a `schema.patch` file in this package, since the output
of `diesel-cli` needs to be tweaked a little for our purposes. For example,
it currently does not support printing materialized views in the same way as
regular tables, so we have to manually add them to the schema file.

If you need to update the patch file, you can do so by following these steps:

1. prefix `patch_file = "src/schema.patch"` in `diesel.toml` with a `#` to comment it out.
2. use `diesel print-schema` and save the output to `src/schema.rs`
3. use `cp src/schema.rs src/schema.rs.orig` to create a backup of the original file
4. use `patch src/schema.rs src/schema.patch` to apply the patch file and solve remaining issues in the `src/schema.rs` file
5. use `diff -Naur --label original --label patched src/schema.rs.orig src/schema.rs` to generate the new content for the `src/schema.patch` file
6. enable the `patch_file` option in the `diesel.toml` file again.
