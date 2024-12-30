# crates_io_team_repo

The code in this package interacts with the
<https://github.com/rust-lang/team/> repository.

The `TeamRepo` trait is used to abstract away the HTTP client for testing
purposes. The `TeamRepoImpl` struct is the actual implementation of
the trait.

If the `mock` feature is enabled, a `MockTeamRepo` struct is available,
which can be used for testing purposes. This struct is generated automatically
by the [`mockall`](https://docs.rs/mockall) crate.
