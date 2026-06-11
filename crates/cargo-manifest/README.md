cargo-manifest
==============================================================================

[`serde`](https://serde.rs) definitions to read and write
[`Cargo.toml`](https://doc.rust-lang.org/cargo/reference/manifest.html) files.


Description
------------------------------------------------------------------------------

This Rust crate contains various structs and enums to represent the contents of
a `Cargo.toml` file. These definitions can be used with [`serde`](https://serde.rs)
and the [`toml`](https://crates.io/crates/toml) crate to read and write
`Cargo.toml` manifest files.

This crate also to some degree supports post-processing of the data to emulate
Cargo's workspace inheritance and `autobins` features. This is used for example
by crates.io to extract whether a crate contains a library or executable
binaries.

> [!NOTE]
> The cargo team regularly adds new features to the `Cargo.toml` file
> definition. This crate aims to keep up-to-date with these changes. You should
> keep this crate up-to-date to correctly parse all fields in modern
> `Cargo.toml` files.


Installation
------------------------------------------------------------------------------

```sh
cargo add cargo-manifest
```


Usage
------------------------------------------------------------------------------

```rust
use cargo_manifest::Manifest;

let manifest = Manifest::from_path("Cargo.toml").unwrap();
```

see [docs.rs](https://docs.rs/cargo-manifest) for more information.


Users
------------------------------------------------------------------------------

- [cargo-chef](https://crates.io/crates/cargo-chef)
- [crates.io](https://github.com/rust-lang/crates.io) is using this crate for
  server-side validation of `Cargo.toml` files.


Alternatives
------------------------------------------------------------------------------

This crate is a fork of the [`cargo_toml`](https://crates.io/crates/cargo_toml)
project. There are only some minor differences between these projects at this
point, you will need to evaluate which one fits your needs better.

There is also [`cargo-util-schemas`](https://crates.io/crates/cargo-util-schemas)
now, which is maintained by the cargo team themselves. This crate was extracted
from the cargo codebase and is used inside the `cargo` binary itself. It is
kept up-to-date with the latest changes to the `Cargo.toml` file format, but is
currently lacking some of the post-processing features that `cargo-manifest`
provides.


License
------------------------------------------------------------------------------

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)

- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
