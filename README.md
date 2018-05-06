# conduit-hyper

[![Build Status](https://travis-ci.org/jtgeibel/conduit-hyper.svg?branch=master)](https://travis-ci.org/jtgeibel/conduit-hyper)

This crate integrates a `hyper 0.12` server with a `conduit 0.8` application
stack.

## Error and Panic Handling

If the application handler returns an `Err(_)` the server will log a message to
stderr and return a generic 500 status response.

If the handler panics, the thread pool prints a message to stderr and the
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
noteworth behavior:

* `scheme` always returns Http as https is not currently directly supported
* `remote_addr` always returns 0.0.0.0:0
* `host` returns an empty string if the `Host` header is not valid UTF-8

There is currently a bug where the `all` method of `headers` may include
duplicate keys multiple times.  See [this issue] for further details.

All other methods on `Request` should behave as expected.

[this issue]: https://github.com/hyperium/http/issues/199

## License

Licensed under either of these:

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   https://opensource.org/licenses/MIT)
