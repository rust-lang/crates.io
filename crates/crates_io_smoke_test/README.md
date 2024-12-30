# crates_io_smoke_test

This package contains a basic smoke test for the <https://staging.crates.io>
environment. It uses the API to fetch the metadata of a test crate, publishes
a new patch version, attempts to download the crate file, and checks that the
git and sparse indexes contain the new version.

Note that a valid `CARGO_REGISTRY_TOKEN` environment variable is required to
run the smoke test. This token must have the `publish` permission for the test
crate. If `--skip-publish` is passed, the smoke test will not publish a new
version of the test crate.
