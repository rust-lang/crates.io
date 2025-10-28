# aws-ip-ranges

<https://ip-ranges.amazonaws.com/ip-ranges.json> as const structs

## Description

If the data from <https://ip-ranges.amazonaws.com/ip-ranges.json> is used in a
Rust program, it can be slow to fetch it every time. This crate provides the
data as const structs, so it can be compiled into the program.

An hourly GitHub Actions workflow updates the data in this crate automatically
and releases a new version of the crate if the data has changed.

## Usage

```rust
fn main() {
  let ip_ranges = aws_ip_ranges::IP_RANGES;
  println!("{ip_ranges:#?}");
}
```

## License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)

- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
