# crates.io

Source code for the default registry for Cargo users. Can be found online at
[crates.io][crates-io]

[crates-io]: https://crates.io

## Development Setup

* `git clone` this repository
* `npm install`
* `npm install -g ember-cli`
* `npm install -g bower && bower install`

## Making UI tweaks or changes

This website is built using [Ember.js](http://emberjs.com/) for the frontend,
which enables tweaking the UI of the site without actually having the server
running locally. To get up and running with just the UI, run:

```
npm run start:staging
```

This will give you a local server to browse while using the staging backend
(hosted on heroku at https://staging-crates-io.herokuapp.com).

If you'd like to run the server with a specific different backend, you can specify specific arguments to `npm start`. For example you can set the proxy to `https://crates.io/` to use the live instance, but do be aware that any modifications made here will be permanent! To do this, run:

```
npm start -- --proxy https://crates.io
```

The same is also available as:

```
npm run start:live
```

This requires NPM 2.0.

## Working on the backend

If you'd like to change the API server (the Rust backend), then the setup is a
little more complicated.

1. Copy the `.env.sample` file to `.env` and change any applicable values as
    directed by the comments in the file. Make sure the values in your new
    `.env` are exported in the shell you use for the following commands.

2. Set up the git index

    ```
    ./script/init-local-index.sh
    ```

    But *do not* modify your `~/.cargo/config` yet. Do that after step 3.

3. Build the server.

    ```
    cargo build
    ```

    On OS X 10.11, you will need to install the openssl headers first, and tell
    cargo where to find them. See https://github.com/sfackler/rust-openssl#osx.

4. Run the migrations

    ```
    ./target/debug/migrate
    ```

5. Run the servers

    ```
    # In one window, run the api server
    ./target/debug/server

    # In another window run the ember-cli server
    npm run start:local
    ```

## Running Tests

1. Configure the location of the test database. Note that this should just be a
   blank database, the test harness will ensure that migrations are run.

    ```
    export TEST_DATABASE_URL=...
    ```

2. Set the s3 bucket to `alexcrichton-test`. No actual requests to s3 will be
   made; the requests and responses are recorded in files in
   `tests/http-data` and the s3 bucket name needs to match the requests in the
   files.

    ```
    export S3_BUCKET=alexcrichton-test
    ```

3. Run the API server tests

    ```
    cargo test
    ```

4. Install [phantomjs](http://phantomjs.org/)

5. Run frontend tests

    ```
    ember test
    ember test --server
    ```

## Tools

For more information on using ember-cli, visit
[http://iamstef.net/ember-cli/](http://ember-cli.com/).

For more information on using cargo, visit
[doc.crates.io](http://doc.crates.io/).

## Deploying a Mirror

**DISCLAIMER: The process of setting up a mirror is a work-in-progress and is
likely to change. It is not currently recommended for mission-critical
production use. It also requires a version of cargo newer than 0.13.0-nightly
(f09ef68 2016-08-02); the version of cargo currently on rustc's beta channel
fulfils this requirement and will be shipped with rustc 1.12.0 scheduled to be
released on 2016-09-29.**

### Current functionality: a read-only, download-API-only mirror

This mirror will function as a read-only duplicate of crates.io's API. You will
be able to download crates using your index and your mirror, but the crate files
will still come from crates.io's S3 storage.

Your mirror will not:

- Allow users to sign up/sign in
- Allow crate publish
- Keep track of any statistics
- Display available crates in its UI

### API server setup

To deploy the API on Heroku, use this button:

[![Deploy](https://www.herokucdn.com/deploy/button.svg)][deploy]

[deploy]: https://heroku.com/deploy

You do not need to fill in any of the optional fields.

### Index mirror setup

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

### Cargo setup

**NOTE: The following configuration requires a cargo version newer than
0.13.0-nightly (f09ef68 2016-08-02). The version of cargo that comes with rust
1.12.0 fulfils this requirement; this version is currently on the beta channel
and is scheduled to be released on 2016-09-29.**

In the project where you want to use your mirror, change your `.cargo/config`
to replace the crates.io source to point to your crates index:

```toml
[source]

[source.mirror]
registry = "https://[host and path to your git server]/crates.io-index"

[source.crates-io]
replace-with = "mirror"
```
