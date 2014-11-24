# crates.io

Source code for the default registry for Cargo users. Can be found online at
[crates.io][crates-io]

[crates-io]: https://crates.io

## Installation

* `git clone` this repository
* `npm install`
* `bower install`

## Configuration

The app currently requires some configuration via environment variables to start
up and run.

```
# Credentials for uploading packages to S3
export S3_BUCKET=...
export S3_ACCESS_KEY=...
export S3_SECRET_KEY=...
export S3_REGION=...

# Credentials for talking to github
export GH_CLIENT_ID=...
export GH_CLIENT_SECRET=...

# Key to sign cookies with
export SESSION_KEY=...

# Location of the *postgres* database
export DATABASE_URL=...

# Remote and local locations of the registry index
export GIT_REPO_URL=file://`pwd`/tmp/index-bare
export GIT_REPO_CHECKOUT=`pwd`/tmp/index-co
```

To set up the git index, run `./script/init-local-index.sh`.

## Running

To run the app, you need to run both the API server and the ember frontend
server.

* `cargo build && ./target/server`
* `ember server --proxy http://localhost:8888`
* Visit the app at [http://localhost:4200](http://localhost:4200).

## Initialize the database

To initialize the database (or run any recent migrations):

```
./target/migrate
```

## Running Tests

* `TEST_DATABASE_URL=... cargo test`

JS tests (note these are not written yet)

* `ember test`
* `ember test --server`

* `export TEST_DATABASE_URL=cargo.test`
* `cargo test`

## Building

* `cargo build`
* `ember build`

## Github application

If you want to login with Github, you need to set your Github application's
callback url to `http://localhost:4200/authorize/github`.

## Amazon AWS S3 Buckets

Buckets should be created in the US Standard region. Other regions are known to cause errors.

For more information on using ember-cli, visit [http://iamstef.net/ember-cli/](http://iamstef.net/ember-cli/).
