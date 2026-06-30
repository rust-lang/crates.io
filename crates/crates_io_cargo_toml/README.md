`crates_io_cargo_toml`
==============================================================================

[`serde`](https://serde.rs) definitions to read and write
[`Cargo.toml`](https://doc.rust-lang.org/cargo/reference/manifest.html) files.


Description
------------------------------------------------------------------------------

This crate contains structs and enums to represent the contents of a
`Cargo.toml` file. These definitions can be used with [`serde`](https://serde.rs)
and the [`toml`](https://crates.io/crates/toml) crate to read and write
`Cargo.toml` manifest files.

It also supports some post-processing of the data to emulate Cargo's workspace
inheritance and `autobins` features. crates.io uses this to extract whether a
crate contains a library or executable binaries.

This is a vendored copy of the
[`cargo-manifest`](https://github.com/LukeMathWalker/cargo-manifest) crate,
simplified for crates.io's needs. It only parses the fields that are relevant to
crates.io rather than aiming to cover every field of a `Cargo.toml` file.


Usage
------------------------------------------------------------------------------

```rust
use crates_io_cargo_toml::Manifest;

let manifest = Manifest::from_path("Cargo.toml").unwrap();
```
