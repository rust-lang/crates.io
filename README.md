# crates.io

Source code for the default [Cargo](http://doc.crates.io) registry. Viewable 
online at [crates.io](https://crates.io).

This project is built on ember-cli and cargo, visit
[iamstef.net/ember-cli](http://ember-cli.com/) or
[doc.crates.io](http://doc.crates.io/) respectively for more information.

## Working on the Frontend

```bash
git clone https://github.com/rust-lang/crates.io.git
cd crates.io/
npm install -g yarn # if you don't have yarn
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
for tests in `app/mirage/fixtures`. Note that the fixture data does not contain
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

After cloning the repo, steps for setting up the backend API server are as 
follows:

1. Copy the `.env.sample` file to `.env` and change any applicable values as
    directed by the comments in the file. Make sure the values in your new
    `.env` are exported in the shell you use for the following commands.

2. Set up the git index:

    ```
    ./script/init-local-index.sh
    ```

    But *do not* modify your `~/.cargo/config` yet. Do that after step 3.

3. Build the server:

    ```
    cargo build
    ```

    On OS X 10.11, you will need to install the openssl headers first, and tell
    cargo where to find them. See https://github.com/sfackler/rust-openssl#osx.

4. Run the migrations:

    ```
    ./target/debug/migrate
    ```

5. Start the backend server:

    ```
    ./target/debug/server
    ```

6. **Optionally** start a local frontend:

    ```
    yarn run start:local
    ```

### Running Tests

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

3. Run the backend API server tests:

    ```
    cargo test
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
registry = 'https://doesnt-matter-but-must-be-present'
```

Once [rust-lang/cargo#3089](https://github.com/rust-lang/cargo/pull/3089) is
released, it won't be necessary to specify a registry URL for a source being
replaced.
