# crates_io_fastly

This package implements functionality for interacting with the Fastly API.

The `Fastly` struct provides methods for purging cached content on Fastly's CDN.
It uses the `reqwest` crate to perform HTTP requests to the Fastly API and
authenticates using an API token.

The main operations supported are:
- `purge()` - Purge a specific path on a single domain
- `purge_both_domains()` - Purge a path on both the primary and prefixed domains

Note that wildcard invalidations are not supported by the Fastly API.
