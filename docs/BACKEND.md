# Backend Overview

## Server

The code to actually run the server is in _src/bin/server.rs_. This is where most of the pieces of
the system are instantiated and configured, and can be thought of as the "entry point" to crates.io.

The server does the following things:

1. Initialize logging
2. Check out the index git repository, if it isn't already checked out
3. Reads values from environment variables to configure a new instance of `crates_io::App`
4. Adds middleware to the app by calling `crates_io::middleware`
5. Syncs the categories defined in _src/categories.toml_ with the categories in the database
6. Starts a [hyper] server that uses the `crates_io::App` instance
7. Tells Nginx on Heroku that the application is ready to receive requests, if running on Heroku
8. Blocks forever (or until the process is killed)

[hyper]: https://crates.io/crates/hyper

## Routes

The API URLs that the server responds to (aka "routes") are defined in
_src/router.rs_.

All of the `api_router` routes are mounted under the `/api/v1` path (see the
lines that look like `router.get("/api/v1/*path", R(api_router.clone()));`).

Each API route definition looks like this:

```rust
api_router.get("/crates", C(krate::index));
```

This line defines a route that responds to a GET request made to
`/api/v1/crates` with the results of calling the `krate::index` function. `C`
is a struct that holds a function and implements the [`conduit::Handler`][]
trait so that the results of the function are the response if the function
succeeds, and that the server returns an error response if the function doesn't
succeed. The `C` struct's purpose is to reduce some boilerplate.

[`conduit::Handler`]: https://docs.rs/conduit/0.8.1/conduit/trait.Handler.html

## Code having to do with running a web application

These modules could _maybe_ be refactored into another crate. Maybe not. But their primary purpose
is supporting the running of crates.io's web application parts, and they don't have much to do with
the crate registry purpose of the application.

### The `app` module

This contains the `App` struct, which holds a `Config` instance plus a few more application
components such as:

- The database connection pools (there are two until we finish migrating the app to use Diesel
  everywhere)
- The GitHub OAuth configuration
- The cookie session key given to [conduit-cookie][]
- The `git2::Repository` instance for the index repo checkout
- The `Config` instance

This module also contains `AppMiddleware`, which implements the `Middleware` trait in order to
inject the `app` instance into every request. That way, we can call `req.app()` to get to any of
these components.

[conduit-cookie]: https://crates.io/crates/conduit-cookie

### The `config` module

### The `db` module

### The `dist` module

### The `http` module

### The `model` module

### The `schema` module

### The `utils` module

## Code having to do with managing a registry of crates

These modules are specific to the domain of being a crate registry. These concepts would exist no
matter what language or framework crates.io was implemented in.

### The `krate` module

### The `users` module

### The `badge` module

### The `categories` module

### The `category` module

### The `dependency` module

### The `download` module

### The `git` module

### The `keyword` module

### The `owner` module

### The `upload` module

### The `uploaders` module

### The `version` module

## Database

## Tests

### Integration tests

A suite of integration tests that require a full crates.io app to be
instantiated and available live in `src/tests`. These are a mixture of tests
that exercise routes and controllers like normal API consumers, and other tests
that require a full blown application and database to be available.

Tests that interact with HTTP services — for example, S3 to upload packages or
index entries — do so by routing requests through a HTTP proxy configured within
`TestApp`. This proxy implements similar behaviour to the [`vcr` Ruby
gem](https://github.com/vcr/vcr): by default, requests are not forwarded to the
actual upstream service, but instead the request is checked against a
"cassette", which is a JSON file in `src/tests/http-data` that contains the
expected request and response. If the request matches, then the saved response
is returned. If the request doesn't match, then an error is returned and the
test fails.

#### Updating test cassettes

When updating integration tests that make HTTP requests, you may need to update
the test cassette associated with that test. This is controlled by the `RECORD`
environment variable: when set to `yes`, the request will actually be forwarded
to the upstream service, and the cassette will be updated with the new response.

In addition to setting `RECORD`, you will also need to set a number of other
environment variables to handle any requests to S3. These are in `.env.sample`,
but are also reproduced here for convenience:

- `TEST_S3_BUCKET`: the S3 bucket used for package uploads.
- `TEST_S3_REGION`: the S3 region used to package uploads. This may also be an
  absolute URL (such as `http://127.0.0.1:19000`), in which that will be used as
  the endpoint. If this is an S3 region name, then it must be one that supports
  what [AWS documents as `s3-Region` host names][s3-region].
- `TEST_S3_INDEX_BUCKET`: the S3 bucket used for sparse index uploads.
- `TEST_S3_INDEX_REGION`: the S3 region used for sparse index uploads. This has
  the same semantics as `TEST_S3_REGION`.
- `TEST_AWS_ACCESS_KEY`: the AWS access key used with S3.
- `TEST_AWS_SECRET_KEY`: the AWS secret key used with S3.

Note that, if the bucket and region environment variables are set, they _must_
match the values used when `src/tests/http-data` was regenerated for existing
tests to pass.

In practice, we use [Minio](https://min.io/) to mock the S3 API when generating
test cassettes, since this means we don't have to share S3 credentials or pay
for an actual bucket. You can run Minio locally on port 19000 with `crates-test`
and `crates-index-test` buckets to match the existing test cassettes. To do this
in Docker with the access key `minio` and secret key `miniominio`, run the
following command:

```sh
docker run --rm \
    -p 19000:9000 -p 19001:9001 \
    -e MINIO_ROOT_USER=minio -e MINIO_ROOT_PASSWORD=miniominio \
    --entrypoint /bin/sh \
    quay.io/minio/minio \
    -c 'mkdir /data/crates-test /data/crates-index-test && exec env minio server /data --console-address :9001'
```

This will give you an S3 compatible Minio instance listening on port 19000, and
a management console on port 19001.

#### Running tests against S3

If you want to run the test suite against an actual S3 bucket, you must do so
with the `RECORD` environment variable set to `yes`, otherwise tests will fail
before any requests are actually sent because the URL in the test cassette
doesn't match the request URL. For example:

```sh
RECORD=yes \
    TEST_S3_REGION=us-east-1 TEST_S3_BUCKET=my-test-bucket \
    TEST_S3_INDEX_REGION=us-east-1 TEST_S3_INDEX_BUCKET=my-test-bucket \
    TEST_AWS_ACCESS_KEY=an-actual-AWS-key TEST_AWS_SECRET_KEY=an-actual-secret-key \
    cargo test
```

Please do not commit any updated test cassettes generated as a result of running
the test suite against S3.

## Scripts

[s3-region]: https://docs.aws.amazon.com/AmazonS3/latest/userguide/VirtualHosting.html#s3-dash-region
