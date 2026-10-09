# `crates_io_zulip`

This package implements functionality for interacting with the Zulip API.

It contains a `ZulipClient` trait that defines the supported operations that
the crates.io codebase needs to interact with Zulip. The `RealZulipClient`
struct is an implementation of this trait that uses the `reqwest` crate to
perform the actual HTTP requests, authenticated as a Zulip bot.

If the `mock` feature is enabled, a `MockZulipClient` struct is available,
which can be used for testing purposes. This struct is generated automatically
by the [`mockall`](https://crates.io/crates/mockall) crate.

The `send_message` example can be used to send a message to a real Zulip
instance:

```sh
ZULIP_API_KEY=... cargo run -p crates_io_zulip --example send_message -- \
  --bot-email crates-io-bot@rust-lang.zulipchat.com \
  --channel t-crates-io \
  --topic "index squashing" \
  "Hello from crates.io!"
```
