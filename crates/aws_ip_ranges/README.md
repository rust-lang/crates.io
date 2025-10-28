# aws-ip-ranges

CloudFront IP ranges from <https://ip-ranges.amazonaws.com/ip-ranges.json>

## Description

This crate provides AWS CloudFront IP ranges as a compile-time constant array,
eliminating the need to fetch and parse the data at runtime.

An hourly GitHub Actions workflow updates the data in this crate automatically
and commits any changes directly to the main branch.

## Usage

```rust
fn main() {
    for cidr in aws_ip_ranges::CLOUDFRONT_CIDRS {
        println!("{}", cidr);
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
