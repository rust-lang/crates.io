# crates_io_heroku

This package contains utilities for accessing Heroku-specific environment
variables.

When the `runtime-dyno-metadata` Heroku Labs feature is enabled, Heroku
exposes application and environment information through environment variables.
This crate provides convenient functions to access these values.

For more information, see:
<https://devcenter.heroku.com/articles/dyno-metadata>
