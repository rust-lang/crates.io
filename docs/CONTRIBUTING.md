# Contributing to Crates.io

## Finding an issue to work on

We try to keep a variety of issues tagged with
[E-mentor](https://github.com/rust-lang/crates.io/issues?q=is%3Aopen+is%3Aissue+
 label%3AE-mentor). These issues should contain, somewhere within the body or
the comments, clear instructions on what needs to be done and where the changes
will need to be made. If any E-mentor issues do not contain this information or
the information isn't clear, we consider this a bug, please comment and ask for
clarification! Please don't hesitate to ask any questions on these issues,
we're paying special attention to them and someone will get back to you with
help as soon as possible.

We'd also love contributions for issues not tagged E-mentor, they just might
not be as well specified. You may want to browse through the labels that start
with A-, which stands for "area", to find issues that match with your interests.

If you'd like to work on something that isn't in a current issue, especially if
it would be a big change, please open a new issue for discussion!

## Setting up a development environment

### Working on the Frontend

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

#### Running Tests

Install [phantomjs](http://phantomjs.org/), typically: `npm install
phantomjs-prebuilt`.

Then run the tests with:

```
yarn run ember test
yarn run ember test --server
```

### Working on the Backend

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

#### Running Tests

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

#### Hosting crates.io locally

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
