# crates.io

[![Build Status](https://travis-ci.org/rust-lang/crates.io.svg?branch=master)](https://travis-ci.org/rust-lang/crates.io)

Source code for the default [Cargo](http://doc.crates.io) registry. Viewable
online at [crates.io](https://crates.io).

This project is built on ember-cli and cargo, visit
[iamstef.net/ember-cli](http://ember-cli.com/) or
[doc.crates.io](http://doc.crates.io/) respectively for more information.

## Working on the Frontend

Install [Yarn](https://yarnpkg.com), see
[yarnpkg.com/en/docs/install](https://yarnpkg.com/en/docs/install) for
instructions for your OS.

```bash
git clone https://github.com/rust-lang/crates.io.git
cd crates.io/
yarn
yarn run bower install
```

The website's frontend is built with [Ember.js](http://emberjs.com/). This
makes it possible to work on the frontend without running a local backend.
To start the frontend run:

```bash
yarn run start:staging
```

This will run a local frontend using the staging backend (hosted on Heroku at
[staging-crates-io.herokuapp.com](https://staging-crates-io.herokuapp.com)).

If you want to set up a particular situation, you can edit the fixture data used
for tests in `mirage/fixtures`. Note that the fixture data does not contain
JSON needed to support every page, so some pages might not load correctly. To
run the frontend and use that data, don't specify any backend:

```bash
yarn run start
```

If you'd like to run the frontend with a specific backend endpoint, you can
specify arguments to `yarn start`. For example you can set the proxy to
`https://crates.io/` to use the production version.

**Note: it is also possible to make changes to the production data**

To do this, run:

```bash
yarn start -- --proxy https://crates.io
# or
yarn run start:live
```

**Note**: This requires npm 2.

### Running Tests

Install [phantomjs](http://phantomjs.org/), typically: `npm install
phantomjs-prebuilt`.

Then run the tests with:

```
yarn run ember test
yarn run ember test --server
```

## Working on the Backend

Working on the backend requires a usable postgres server and to configure
crates.io to use it. There are slight differences in configuration for
hosting the backend and running tests, both of which are described in more
details in the appropriate subsections.

After cloning the repo, do the following:

1. Install [Postgres](https://www.postgresql.org/) >= 9.5. On Linux this is
   generally available in the distribution repositories as `postgresql` or
   `postgresql-server`. This will need to be up and running for running tests
   for hosting the site locally.

2. Copy the `.env.sample` file to `.env`. Some settings will need to be
   modified. These instructions are in the subsequent sections.

3. Install `diesel_cli` using `cargo install diesel_cli --no-default-features
   --features postgres`.

### Running Tests

After following the above instructions:

1. Configure the location of the test database. Create a database specifically
   for testing since running the tests will clear the database. For example,
   to use a database named `cargo_registry_test`, create it in postgres by
   running `psql` to connect to postgres, then run `CREATE DATABASE
   cargo_registry_test;`. The test harness will ensure that migrations are run.

   In your `.env` file, specify your test database URL. Here's an example,
   assuming your test database is named `cargo_registry_test`:

   ```
   export TEST_DATABASE_URL=postgres://postgres@localhost/cargo_registry_test
   ```

2. Run the backend API server tests:

   ```
   cargo test
   ```

### Hosting crates.io locally

After following the instructions described in "Working on the Backend":

1. Make sure your local postgres instance is running and create a database for
   use with the local crates.io instance. `cargo_registry` is a good name to
   use. You can do this by running `psql` to connect to `postgres` and run:

   ```
   CREATE DATABASE cargo_registry;
   ```

2. Modify the `.env` configuration file's `DATABASE_URL` setting to point
   to the local postgres instance with the database you want to use. If you've
   followed these instructions it should likely look like:

   ```
   export DATABASE_URL=postgres://postgres@localhost/cargo_registry
   ```

3. Set up the git index:

   ```
   ./script/init-local-index.sh
   ```

   But *do not* modify your `~/.cargo/config` yet (but record the instructions
   shown at the end of this step as you'll need them later).

4. Build the server:

   ```
   cargo build
   ```

   On OS X 10.11, you will need to install the openssl headers first, and tell
   cargo where to find them. See https://github.com/sfackler/rust-openssl#osx.

5. Modify your `~/.cargo/config` after successfully building crates.io
   following the instructions shown at the end of Step 3.

5. Run the migrations:

   ```
   diesel migration run
   ```

6. Start the backend server:

   ```
   ./target/debug/server
   ```

7. **Optionally** start a local frontend:

   ```
   yarn run start:local
   ```

## Categories

The list of categories available on crates.io is stored in
`src/categories.toml`. To propose adding, removing, or changing a category,
send a pull request making the appropriate change to that file as noted in the
comment at the top of the file. Please add a description that will help others
to know what crates are in that category.

For new categories, it's helpful to note in your PR description examples of
crates that would fit in that category, and describe what distinguishes the new
category from existing categories.

After your PR is accepted, the next time that crates.io is deployed the
categories will be synced from this file.

## Deploying & Using a Mirror

**DISCLAIMER: The process of setting up a mirror is a work-in-progress and is
likely to change. It is not currently recommended for mission-critical
production use. It also requires Cargo from Rust distribution 1.12.0 or
later.**

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

The only config variable you need to set is `GIT_REPO_URL`, which should be the
git URL of your crates index repository; see the next section for setup
instructions for that.

### Index Mirror Setup

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

### Local Cargo Setup

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
