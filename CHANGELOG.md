# Changelog

## v0.4.1 (2021-11-11)

#### :house: Internal
* [#34](https://github.com/conduit-rust/conduit-hyper/pull/34) ConduitRequest: Inline `parts()` method ([@Turbo87](https://github.com/Turbo87))
* [#33](https://github.com/conduit-rust/conduit-hyper/pull/33) RequestInfo: Replace custom struct with `Request<Bytes>` ([@Turbo87](https://github.com/Turbo87))

#### Committers: 1
- Tobias Bieniek ([@Turbo87](https://github.com/Turbo87))


## v0.4.0 (2021-11-06)

#### :boom: Breaking Change
* [#30](https://github.com/conduit-rust/conduit-hyper/pull/30) Update conduit crates to 0.10.0 (minor) ([@renovate[bot]](https://github.com/apps/renovate))

#### :house: Internal
* [#31](https://github.com/conduit-rust/conduit-hyper/pull/31) ConduitRequest: Remove obsolete `extensions` field ([@Turbo87](https://github.com/Turbo87))

#### Committers: 1
- Tobias Bieniek ([@Turbo87](https://github.com/Turbo87))


## v0.3.0 (2021-11-06)

#### :rocket: Enhancement
* [#29](https://github.com/conduit-rust/conduit-hyper/pull/29) Update `conduit` to v0.9.0 ([@Turbo87](https://github.com/Turbo87))
* [#26](https://github.com/conduit-rust/conduit-hyper/pull/26) Declare minimum supported Rust version ([@Turbo87](https://github.com/Turbo87))
* [#14](https://github.com/conduit-rust/conduit-hyper/pull/14) Use `tracing-subscriber` instead of `log` and `env_logger` ([@Turbo87](https://github.com/Turbo87))

#### :house: Internal
* [#28](https://github.com/conduit-rust/conduit-hyper/pull/28) ServiceError: Use `thiserror` to derive traits ([@Turbo87](https://github.com/Turbo87))
* [#27](https://github.com/conduit-rust/conduit-hyper/pull/27) Improve CI setup ([@Turbo87](https://github.com/Turbo87))
* [#25](https://github.com/conduit-rust/conduit-hyper/pull/25) CI: Remove obsolete rustup steps ([@Turbo87](https://github.com/Turbo87))
* [#19](https://github.com/conduit-rust/conduit-hyper/pull/19) Add `Cargo.lock` file ([@Turbo87](https://github.com/Turbo87))
* [#18](https://github.com/conduit-rust/conduit-hyper/pull/18) CI: Use `Swatinem/rust-cache` for caching ([@Turbo87](https://github.com/Turbo87))
* [#16](https://github.com/conduit-rust/conduit-hyper/pull/16) Fix `needless_borrow` warnings ([@JohnTitor](https://github.com/JohnTitor))

#### Committers: 2
- Tobias Bieniek ([@Turbo87](https://github.com/Turbo87))
- Yuki Okushi ([@JohnTitor](https://github.com/JohnTitor))


## v0.3.0-alpha.6 (2020-12-24)

#### :rocket: Enhancement
* [#13](https://github.com/conduit-rust/conduit-hyper/pull/13) Upgrade to `tokio 1.0` and `hyper 0.14`  ([@jtgeibel](https://github.com/jtgeibel))

#### Committers: 1
- Justin Geibel ([@jtgeibel](https://github.com/jtgeibel))


## v0.3.0-alpha.5 (2020-07-07)

#### :rocket: Enhancement
* [#12](https://github.com/conduit-rust/conduit-hyper/pull/12) Add path rewriting support ([@jtgeibel](https://github.com/jtgeibel))

#### Committers: 1
- Justin Geibel ([@jtgeibel](https://github.com/jtgeibel))


## v0.3.0-alpha.4 (2020-05-26)

#### :house: Internal
* [#11](https://github.com/conduit-rust/conduit-hyper/pull/11) Remove unused code ([@JohnTitor](https://github.com/JohnTitor))

#### Committers: 1
- Yuki Okushi ([@JohnTitor](https://github.com/JohnTitor))


## v0.3.0-alpha.3 (2020-05-24)

#### :boom: Breaking Change
* [#10](https://github.com/conduit-rust/conduit-hyper/pull/10) Remove url path normalization logic ([@jtgeibel](https://github.com/jtgeibel))
* [#9](https://github.com/conduit-rust/conduit-hyper/pull/9) Remove rejection of requests when over capacity ([@jtgeibel](https://github.com/jtgeibel))

#### :rocket: Enhancement
* [#8](https://github.com/conduit-rust/conduit-hyper/pull/8) Depend on `futures-util` instead of `futures` ([@JohnTitor](https://github.com/JohnTitor))

#### Committers: 2
- Justin Geibel ([@jtgeibel](https://github.com/jtgeibel))
- Yuki Okushi ([@JohnTitor](https://github.com/JohnTitor))


## v0.3.0-alpha.2 (2020-03-03)

#### :rocket: Enhancement
* [#7](https://github.com/conduit-rust/conduit-hyper/pull/7) Add support for changes to `conduit::Body` ([@jtgeibel](https://github.com/jtgeibel))

#### Committers: 1
- Justin Geibel ([@jtgeibel](https://github.com/jtgeibel))


## v0.3.0-alpha.1 (2020-02-29)

#### :boom: Breaking Change
* [#6](https://github.com/conduit-rust/conduit-hyper/pull/6) Bump to latest conduit alpha and use `http` types ([@jtgeibel](https://github.com/jtgeibel))

#### Committers: 1
- Justin Geibel ([@jtgeibel](https://github.com/jtgeibel))


## v0.2.0-alpha.4 (2020-01-15)

#### :house: Internal
* [#5](https://github.com/conduit-rust/conduit-hyper/pull/5) Enable Github Actions ([@jtgeibel](https://github.com/jtgeibel))

#### Committers: 1
- Justin Geibel ([@jtgeibel](https://github.com/jtgeibel))

