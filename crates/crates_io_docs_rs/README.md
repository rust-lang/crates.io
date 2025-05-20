# crates_io_docs_rs

This package implements functionality for interacting with the docs.rs API.

It contains a `DocsRsClient` trait that defines the supported operations, that
the crates.io codebase needs to interact with docs.rs. The `RealDocsRsClient`
struct is an implementation of this trait that uses the `reqwest` crate to
perform the actual HTTP requests.

If the `mock` feature is enabled, a `MockDocsRsClient` struct is available,
which can be used for testing purposes. This struct is generated automatically
by the [`mockall`](https://docs.rs/mockall) crate.
