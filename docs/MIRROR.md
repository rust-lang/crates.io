# Deploying & using a mirror

**DISCLAIMER: The process of setting up a mirror is a work-in-progress and is
likely to change. It is not currently recommended for mission-critical
production use. It also requires Cargo from Rust distribution 1.12.0 or
later.**

## Current functionality: a read-only, download-API-only mirror

This mirror will function as a read-only duplicate of crates.io's API. You will
be able to download crates using your index and your mirror, but the crate files
will still come from crates.io's S3 storage.

Your mirror will not:

- Allow users to sign up/sign in
- Allow crate publish
- Keep track of any statistics
- Display available crates in its UI

## API server setup

To deploy the API on Heroku, use this button:

[![Deploy](https://www.herokucdn.com/deploy/button.svg)][deploy]

[deploy]: https://heroku.com/deploy?template=https://github.com/rust-lang/crates.io

The only config variable you need to set is `GIT_REPO_URL`, which should be the
git URL of your crates index repository; see the next section for setup
instructions for that.

## Index Mirror Setup

You also need a mirror of the crates.io git index, and your index needs to point
to your API server.

1. `git clone https://github.com/rust-lang/crates.io-index.git`
2. Edit the config.json file to point to your API server so it looks like:

    ```json
    {
      "dl": "https://[your heroku app name].herokuapp.com/api/v1/crates",
      "api": "https://[your heroku app name].herokuapp.com/"
    }
    ```

3. Commit and push to wherever you will be hosting your index (ex: github,
    gitlab, an internal git server)

4. In order to keep your mirror index up to date, schedule a `git pull` of the
    official index. How to do this depends on how you are hosting your index,
    but could be done through `cron` or a scheduled CI job, for example.

## Local Cargo Setup

NOTE: The following configuration requires Cargo from Rust 1.12.0
distribution or later.

In the project where you want to use your mirror, change your `.cargo/config`
to replace the crates.io source to point to your crates index:

```toml
[source]

[source.mirror]
registry = "https://[host and path to your git server]/crates.io-index"

[source.crates-io]
replace-with = "mirror"
```
