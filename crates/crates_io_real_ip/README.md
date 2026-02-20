# crates_io_real_ip

Extract real client IP addresses from X-Forwarded-For headers

## Description

This crate provides functionality to extract the real client IP address from
X-Forwarded-For headers when running behind CDN proxies. It filters out
known CDN proxy IPs (CloudFront, Fastly) to find the actual client IP.

The crate includes IP ranges from the following sources as compile-time constants,
eliminating the need to fetch and parse the data at runtime:

- CloudFront: <https://ip-ranges.amazonaws.com/ip-ranges.json>
- Fastly: <https://api.fastly.com/public-ip-list>

An hourly GitHub Actions workflow updates the CDN IP ranges automatically
and commits any changes directly to the main branch.

## Usage

```rust
use crates_io_real_ip::process_xff_headers;
use http::HeaderMap;

fn handle_request(headers: &HeaderMap) {
    if let Some(real_ip) = process_xff_headers(headers) {
        println!("Real client IP: {}", real_ip);
    }
}
```

## License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)

- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
