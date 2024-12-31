# crates_io_index

This package contains the code necessary to interact with the
[crates.io-index repository](https://github.com/rust-lang/crates.io-index).

Specifically, it contains:

- the data structures used to serialize and deserialize the files in the index
- a `Repository` abstraction to perform various operations on the index
- and, for testing purposes, an `UpstreamIndex` struct that can be used to
  create a fake index locally.
