# conduit-hyper

[![Build Status](https://travis-ci.org/jtgeibel/conduit-hyper.svg?branch=master)](https://travis-ci.org/jtgeibel/conduit-hyper)

This crate integrates a `hyper 0.13` server with a `conduit 0.8` application
stack.

## Error and Panic Handling

If the application handler returns an `Err(_)` the server will log the
description via the `log` crate and then return a generic 500 status response.

If the handler panics, the default panic handler prints a message to stderr and the
connnection is closed without sending a response.  In the future, these panics
will likely be turned into a generic 500 status response.

## Request Processing

If the request includes a body, the entire body is buffered before the handler
is dispatched on a thread.  There is currently no restriction on the maximum
body size so a client can consume large amounts of memory by sending a large
body.  Therefore it is recommended to use a reverse proxy which limits the
maximum body size.

Header values that are not valid UTF-8 are replaced with an empty string.

### conduit::Request

The following methods on the `Request` provided to the application have
noteworthy behavior:

* `scheme` always returns Http as https is not currently directly supported
* `host` returns an empty string if the `Host` header is not valid UTF-8

All other methods on `Request` should behave as expected.

## TODO

* Include the `X-Request-Id` header when logging an error

## License

Licensed under either of these:

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   https://opensource.org/licenses/MIT)
