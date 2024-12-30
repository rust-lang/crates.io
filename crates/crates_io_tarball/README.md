# crates_io_tarball

This package is used to extract metadata from a `.crate` file, which is the
format used to distribute Rust libraries on https://crates.io.

The main source of metadata is the `Cargo.toml` file, which must be included in
the `.crate` file.

A secondary source of metadata is the `.cargo_vcs_info.json` file, which
contains information about the version control system that was used at the
time of publishing the crate. Note that this file is optional, and must not be
relied upon for critical information since a malicious user could tamper with
it before publishing the crate.
