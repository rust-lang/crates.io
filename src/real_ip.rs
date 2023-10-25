use http::HeaderValue;
use std::net::IpAddr;
use std::str::from_utf8;

/// Parses the content of an `X-Forwarded-For` header into a
/// `Vec<Result<IpAddr, &[u8]>>`.
#[allow(dead_code)]
fn parse_xff_header(header: &HeaderValue) -> Vec<Result<IpAddr, &[u8]>> {
    let bytes = header.as_bytes();
    if bytes.is_empty() {
        return vec![];
    }

    bytes
        .split(|&byte| byte == b',')
        .map(|bytes| parse_ip_addr(bytes))
        .collect()
}

fn parse_ip_addr(bytes: &[u8]) -> Result<IpAddr, &[u8]> {
    from_utf8(bytes)
        .map_err(|_| bytes)?
        .trim()
        .parse()
        .map_err(|_| bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;

    #[test]
    fn test_parse_xff_header() {
        #[track_caller]
        fn test(input: &'static [u8], expectation: Vec<Result<&str, &[u8]>>) {
            let header = HeaderValue::from_bytes(input).unwrap();

            let expectation: Vec<Result<IpAddr, &[u8]>> = expectation
                .into_iter()
                .map(|ip| ip.map(|ip| ip.parse().unwrap()))
                .collect();

            assert_eq!(parse_xff_header(&header), expectation)
        }

        test(b"", vec![]);
        test(b"1.2.3.4", vec![Ok("1.2.3.4")]);
        test(
            b"1.2.3.4, 11.22.33.44",
            vec![Ok("1.2.3.4"), Ok("11.22.33.44")],
        );
        test(
            b"oh, hi,,127.0.0.1,,,,, 12.34.56.78  ",
            vec![
                Err(b"oh"),
                Err(b" hi"),
                Err(b""),
                Ok("127.0.0.1"),
                Err(b""),
                Err(b""),
                Err(b""),
                Err(b""),
                Ok("12.34.56.78"),
            ],
        );
    }
}
