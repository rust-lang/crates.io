# crates_io_github

This package implements functionality for interacting with the GitHub API.

It contains a `GitHubClient` trait that defines the supported operations, that
the crates.io codebase needs to interact with GitHub. The `RealGitHubClient`
struct is an implementation of this trait that uses the `reqwest` crate to
perform the actual HTTP requests.

If the `mock` feature is enabled, a `MockGitHubClient` struct is available,
which can be used for testing purposes. This struct is generated automatically
by the [`mockall`](https://docs.rs/mockall) crate.
