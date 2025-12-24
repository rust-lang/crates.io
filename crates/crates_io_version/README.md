# crates_io_version

This package provides utilities for determining the currently running version
of the crates.io application.

The version is typically identified by the Git commit SHA of the deployed code.
This crate supports multiple methods for discovering this information, including
Heroku-specific environment variables and other deployment platforms.
