# crates_io_session

This package contains a `SessionExtension` extractor for the
[`axum`](https://docs.rs/axum) web framework and a corresponding
`attach_session()` middleware based on a signed `cargo_session` cookie.
This abstraction allows us to save and retrieve data from the session
cookie in a safe way.
