# crates.io

Source code for the default registry for Cargo users. Can be found online at
[crates.io][crates-io]

[crates-io]: https://crates.io

## Installation

* `git clone` this repository
* `npm install`

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

1. Define some environment variables:

    ```
    # Credentials for uploading packages to S3, these can be blank if you're not
    # publishing locally.
    export S3_BUCKET=...
    export S3_ACCESS_KEY=...
    export S3_SECRET_KEY=...
    export S3_REGION=...      # not needed if the S3 bucket is in US standard

    # Credentials for talking to github, can be blank if you're not logging in.
    #
    # When registering a new application, be sure to set the callback url to the
    # address `http://localhost:4200/authorize/github`.
    export GH_CLIENT_ID=...
    export GH_CLIENT_SECRET=...

    # Key to sign and encrypt cookies with
    export SESSION_KEY=...

    # Location of the *postgres* database
    #
    # e.g. postgres://postgres:@localhost/cargo_registry
    export DATABASE_URL=...

    # Remote and local locations of the registry index
    export GIT_REPO_URL=file://`pwd`/tmp/index-bare
    export GIT_REPO_CHECKOUT=`pwd`/tmp/index-co
    ```

2. Set up the git index

    ```
    ./script/init-local-index.sh
    ```

3. Build the server

    ```
    cargo build
    ```

4. Run the migrations

    ```
    ./target/migrate
    ```

5. Run the servers

    ```
    # In one window, run the api server
    ./target/server

    # In another window run the ember-cli server
    npm run start:local
    ```

## Running Tests

1. Configure the location of the test database. Note that this should just be a
   blank database, the test harness will ensure that migrations are run.

    ```
    export TEST_DATABASE_URL=...
    ```

2. Run the API server tests

    ```
    cargo test
    ```

3. Run frontend tests

    ```
    ember test
    ember test --server
    ```

## Tools

For more information on using ember-cli, visit
[http://iamstef.net/ember-cli/](http://iamstef.net/ember-cli/).

For more information on using cargo, visit
[doc.crates.io](http://doc.crates.io/).
