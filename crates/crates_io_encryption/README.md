# `crates_io_encryption`

This package implements encryption of data at rest in the database.

The current usage is to encrypt/decrypt Oauth tokens; the types are named to indicate they're for
token encryption even though they operate on string slices and bytes. If we need to encrypt data
other than tokens, we should consider renaming the types and/or using types in the function
arguments/return values to ensure the right data is being encrypted in the right way.

All data is being encrypted/decrypted with the same key that comes from environment variables. If
there are use cases for being able to rotate a key and only affect some of the data, we should
rethink the way the code is set up rather than the current hardcoding of env var names (and typing
the functions' inputs and outputs will become more useful in that case).
