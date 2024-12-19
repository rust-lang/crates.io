use http::{HeaderMap, HeaderValue};
use ipnetwork::IpNetwork;
use std::iter::Iterator;
use std::net::IpAddr;
use std::str::from_utf8;
use std::sync::LazyLock;

const X_FORWARDED_FOR: &str = "X-Forwarded-For";

static CLOUD_FRONT_NETWORKS: LazyLock<Vec<IpNetwork>> = LazyLock::new(|| {
    let ipv4_prefixes = aws_ip_ranges::IP_RANGES
        .prefixes
        .iter()
        .filter(|prefix| prefix.service == "CLOUDFRONT")
        .map(|prefix| prefix.ip_prefix);

    let ipv6_prefixes = aws_ip_ranges::IP_RANGES
        .ipv6_prefixes
        .iter()
        .filter(|prefix| prefix.service == "CLOUDFRONT")
        .map(|prefix| prefix.ipv6_prefix);

    ipv4_prefixes
        .chain(ipv6_prefixes)
        .filter_map(|prefix| match prefix.parse() {
            Ok(ip_network) => Some(ip_network),
            Err(error) => {
                warn!(%error, "Failed to parse AWS CloudFront CIDR");
                None
            }
        })
        .collect()
});

fn is_cloud_front_ip(ip: &IpAddr) -> bool {
    CLOUD_FRONT_NETWORKS
        .iter()
        .any(|trusted_proxy| trusted_proxy.contains(*ip))
}

/// Extracts the client IP address from the `X-Forwarded-For` header.
///
/// This function will return the last valid non-CloudFront IP address in the
/// `X-Forwarded-For` header, if any.
pub fn process_xff_headers(headers: &HeaderMap) -> Option<IpAddr> {
    headers
        .get_all(X_FORWARDED_FOR)
        .iter()
        .flat_map(|header| {
            parse_xff_header(header)
                .into_iter()
                .filter_map(|r| r.ok())
                .filter(|ip| !is_cloud_front_ip(ip))
        })
        .next_back()
}

/// Parses the content of an `X-Forwarded-For` header into a
/// `Vec<Result<IpAddr, &[u8]>>`.
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
    fn test_process_xff_headers() {
        #[track_caller]
        fn test(input: Vec<&[u8]>, expectation: Option<&str>) {
            let mut headers = HeaderMap::new();
            for value in input {
                let value = HeaderValue::from_bytes(value).unwrap();
                headers.append(X_FORWARDED_FOR, value);
            }

            let expectation: Option<IpAddr> = expectation.map(|ip| ip.parse().unwrap());

            assert_eq!(process_xff_headers(&headers), expectation)
        }

        // Generic behavior
        test(vec![], None);
        test(vec![b""], None);
        test(vec![b"1.1.1.1"], Some("1.1.1.1"));
        test(vec![b"1.1.1.1, 2.2.2.2"], Some("2.2.2.2"));
        test(vec![b"1.1.1.1, 2.2.2.2, 3.3.3.3"], Some("3.3.3.3"));
        test(
            vec![b"oh, hi,,127.0.0.1,,,,, 12.34.56.78  "],
            Some("12.34.56.78"),
        );

        // CloudFront behavior
        test(vec![b"130.176.118.147"], None);
        test(vec![b"1.1.1.1, 130.176.118.147"], Some("1.1.1.1"));
        test(vec![b"1.1.1.1, 2.2.2.2, 130.176.118.147"], Some("2.2.2.2"));

        // Multiple headers behavior
        test(vec![b"1.1.1.1, 2.2.2.2", b"3.3.3.3"], Some("3.3.3.3"));
        test(
            vec![b"1.1.1.1, 130.176.118.147", b"3.3.3.3"],
            Some("3.3.3.3"),
        );
    }

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
