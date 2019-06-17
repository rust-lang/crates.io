# Backend Overview

## Server

The code to actually run the server is in *src/bin/server.rs*. This is where most of the pieces of
the system are instantiated and configured, and can be thought of as the "entry point" to crates.io.

The server does the following things:

1. Initialize logging
2. Check out the index git repository, if it isn't already checked out
3. Reads values from environment variables to configure a new instance of `cargo_registry::App`
4. Adds middleware to the app by calling `cargo_registry::middleware`
5. Syncs the categories defined in *src/categories.toml* with the categories in the database
6. Starts either a [conduit] or a [hyper] server that uses the `cargo_registry::App` instance
7. Tells Nginx on Heroku that the application is ready to receive requests, if running on Heroku
8. Blocks forever (or until the process is killed)

[civet]: https://crates.io/crates/civet
[hyper]: https://crates.io/crates/hyper

## Routes

The API URLs that the server responds to (aka "routes") are defined in
*src/lib.rs*.

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

These modules could *maybe* be refactored into another crate. Maybe not. But their primary purpose
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

## Scripts
