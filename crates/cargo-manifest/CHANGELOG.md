# Changelog

## v0.19.1 (2025-01-16)

## v0.19.0 (2025-01-15)

### :boom: Breaking Change

- [#66](https://github.com/LukeMathWalker/cargo-manifest/pull/66) Fix impl Default for Manifest ([@hdoordt](https://github/hdoordt))

## v0.18.1 (2025-01-13)

### :bug: Bugfixes

- [#65](https://github.com/LukeMathWalker/cargo-manifest/pull/65) Fix: don't populate 'crate-type' with 'bin' for binaries ([@LukeMathWalker](https://github/LukeMathWalker))

## v0.18.0 (2025-01-10)

### :rocket: Enhancements

- [#64](https://github.com/LukeMathWalker/cargo-manifest/pull/64) Support `package.resolver = "3"`, as shipped in Rust 1.84.0 ([@LawnGnome](https://github/LawnGnome))

## v0.17.0 (2024-11-28)

### :rocket: Enhancements

- [#59](https://github.com/LukeMathWalker/cargo-manifest/pull/59) toml: disable the unused `display` Cargo feature ([@AudaciousAxiom](https://github/AudaciousAxiom))
- [#60](https://github.com/LukeMathWalker/cargo-manifest/pull/60) Add Edition 2024 ([@eth3lbert](https://github/eth3lbert))

## v0.16.1 (2024-11-14)

### :rocket: Enhancements

- [#58](https://github.com/LukeMathWalker/cargo-manifest/pull/58) Edition: Implement `as_str()` fn ([@Turbo87](https://github/Turbo87))

## v0.16.0 (2024-10-31)

### :boom: Breaking Change

- [#57](https://github.com/LukeMathWalker/cargo-manifest/pull/57) Parse workspace metadata ([@hdoordt](https://github/hdoordt))

### :rocket: Enhancements

- [#56](https://github.com/LukeMathWalker/cargo-manifest/pull/56) Add support for `package.autolib` field ([@Turbo87](https://github/Turbo87))

## v0.15.2 (2024-09-05)

### :memo: Documentation

- [#53](https://github.com/LukeMathWalker/cargo-manifest/pull/53) Improve readme document ([@Turbo87](https://github/Turbo87))
- [#54](https://github.com/LukeMathWalker/cargo-manifest/pull/54) Fix rustdoc warnings ([@Turbo87](https://github/Turbo87))
- [#55](https://github.com/LukeMathWalker/cargo-manifest/pull/55) Use `README.md` as module doc comment ([@Turbo87](https://github/Turbo87))

## v0.15.1 (2024-08-22)

### :rocket: Enhancements

- [#50](https://github.com/LukeMathWalker/cargo-manifest/pull/50) Implement legacy library discovery fallback ([@Turbo87](https://github/Turbo87))

### :bug: Bugfixes

- [#51](https://github.com/LukeMathWalker/cargo-manifest/pull/51) Fix duplicate `src/main.rs` discovery ([@Turbo87](https://github/Turbo87))

### :memo: Documentation

- [#52](https://github.com/LukeMathWalker/cargo-manifest/pull/52) Add release changelog ([@Turbo87](https://github/Turbo87))

## v0.15.0 (2024-06-29)

### :rocket: Enhancements

- [#32](https://github.com/LukeMathWalker/cargo-manifest/pull/32) Simplify `Option<Vec<Product>>` to `Vec<Product>` ([@Turbo87](https://github/Turbo87))
- [#36](https://github.com/LukeMathWalker/cargo-manifest/pull/36) Adjust default `crate_type` for libraries ([@Turbo87](https://github/Turbo87))
- [#37](https://github.com/LukeMathWalker/cargo-manifest/pull/37) Set default `crate_type` for binaries, examples, tests, and benchmarks ([@Turbo87](https://github/Turbo87))
- [#38](https://github.com/LukeMathWalker/cargo-manifest/pull/38) Fill explicit `lib` declaration with default values ([@Turbo87](https://github/Turbo87))
- [#40](https://github.com/LukeMathWalker/cargo-manifest/pull/40) AbstractFilesystem: Add doc comments ([@Turbo87](https://github/Turbo87))
- [#41](https://github.com/LukeMathWalker/cargo-manifest/pull/41) Change `autobins` and friends to `Option<bool>` ([@Turbo87](https://github/Turbo87))
- [#43](https://github.com/LukeMathWalker/cargo-manifest/pull/43) Add doc comments to `Package::auto*` fields ([@Turbo87](https://github/Turbo87))
- [#47](https://github.com/LukeMathWalker/cargo-manifest/pull/47) Implement `Package::version()` fn ([@Turbo87](https://github/Turbo87))

### :bug: Bugfixes

- [#48](https://github.com/LukeMathWalker/cargo-manifest/pull/48) Reimplement target auto-discovery ([@Turbo87](https://github/Turbo87))

## v0.14.0 (2024-03-29)

### :rocket: Enhancements

- [#30](https://github.com/LukeMathWalker/cargo-manifest/pull/30) Package version can now be optional ([@markdingram](https://github/markdingram))

## v0.12.1 (2023-10-14)

### :rocket: Enhancements

- [#27](https://github.com/LukeMathWalker/cargo-manifest/pull/27) Dependency: Implement `simplify()` fn ([@Turbo87](https://github/Turbo87))

## v0.12.0 (2023-09-28)

### :rocket: Enhancements

- [#24](https://github.com/LukeMathWalker/cargo-manifest/pull/24) Dependency: Add `Inherited` variant ([@Turbo87](https://github/Turbo87))

## v0.11.1 (2023-09-18)

### :rocket: Enhancements

- [#21](https://github.com/LukeMathWalker/cargo-manifest/pull/21) Skip serialization for `bool` fields if the value matches the default value ([@Turbo87](https://github/Turbo87))
- [#22](https://github.com/LukeMathWalker/cargo-manifest/pull/22) Make `Manifest` and `Package` easier to instantiate ([@Turbo87](https://github/Turbo87))

### :bug: Bugfixes

- [#20](https://github.com/LukeMathWalker/cargo-manifest/pull/20) Package: Fix `default-run` naming ([@Turbo87](https://github/Turbo87))

## v0.11.0 (2023-09-11)

### :rocket: Enhancements

- [#19](https://github.com/LukeMathWalker/cargo-manifest/pull/19) Add split-debuginfo to cargo profiles ([@mladedav](https://github/mladedav))

## v0.10.0 (2023-08-31)

### :rocket: Enhancements

- [#17](https://github.com/LukeMathWalker/cargo-manifest/pull/17) MaybeInherited: Implement `as_ref()` and `local()` fns ([@Turbo87](https://github/Turbo87))
- [#18](https://github.com/LukeMathWalker/cargo-manifest/pull/18) Change `build` field to `Option<StringOrBool>` ([@Turbo87](https://github/Turbo87))

## v0.9.0 (2023-05-20)

### :rocket: Enhancements

- [#16](https://github.com/LukeMathWalker/cargo-manifest/pull/16) Copy strip settings from upstream ([@divergentdave](https://github/divergentdave))

## v0.8.0 (2023-04-06)

## v0.7.1 (2022-12-15)

## v0.7.0 (2022-12-13)

## v0.6.0 (2022-12-10)

## v0.5.0 (2022-12-07)

### :rocket: Enhancements

- [#14](https://github.com/LukeMathWalker/cargo-manifest/pull/14) Fix MaybeInherited repository issue. ([@dessalines](https://github/dessalines))

## v0.4.0 (2022-09-23)

## v0.3.0 (2022-08-24)

### :rocket: Enhancements

- [#13](https://github.com/LukeMathWalker/cargo-manifest/pull/13) [Breaking] Refine serialization 

## v0.2.9 (2022-08-19)

## v0.2.8 (2022-07-01)

### :rocket: Enhancements

- [#12](https://github.com/LukeMathWalker/cargo-manifest/pull/12) Add support for the inherits field. 

## v0.2.7 (2022-06-26)

## v0.2.6 (2021-09-03)

## v0.2.5 (2021-09-02)

## v0.2.4 (2021-04-21)

## v0.2.3 (2021-01-19)

## v0.2.2 (2021-01-02)

