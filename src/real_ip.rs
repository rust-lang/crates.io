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

pub fn process_xff_headers(headers: &HeaderMap) -> Option<IpAddr> {
    let mut xff_iter = headers.get_all(X_FORWARDED_FOR).iter();
    let Some(first_header) = xff_iter.next() else {
        debug!(target: "real_ip", "No X-Forwarded-For header found");
        return None;
    };

    let has_more_headers = xff_iter.next().is_some();
    if has_more_headers {
        // This only happens for requests going directly to crates-io.herokuapp.com,
        // since AWS CloudFront automatically merges these headers into one.
        //
        // The Heroku router has a bug where it currently (2023-10-25) appends
        // the connecting IP to the **first** header instead of the last.
        //
        // In this specific scenario we will read the IP from the first header,
        // instead of the last, to work around the Heroku bug. We also don't
        // have to care about the trusted proxies, since the request was
        // apparently sent to Heroku directly.

        debug!(target: "real_ip", ?first_header, "Multiple X-Forwarded-For headers found, using the first one due to Heroku bug");

        parse_xff_header(first_header)
            .into_iter()
            .filter_map(|r| r.ok())
            .next_back()
    } else {
        // If the request came in through CloudFront we only get a single,
        // merged header.
        //
        // If the request came in through Heroku and only had a single header
        // originally, then we also only get a single header.
        //
        // In this case return the right-most IP address that is not in the list
        // of IPs from trusted proxies (i.e. CloudFront).

        debug!(target: "real_ip", ?first_header, "Single X-Forwarded-For header found");

        parse_xff_header(first_header)
            .into_iter()
            .filter_map(|r| r.ok())
            .filter(|ip| !is_cloud_front_ip(ip))
            .next_back()
    }
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

        // Heroku workaround
        test(vec![b"1.1.1.1, 2.2.2.2", b"3.3.3.3"], Some("2.2.2.2"));
        test(
            vec![b"1.1.1.1, 130.176.118.147", b"3.3.3.3"],
            Some("130.176.118.147"),
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
