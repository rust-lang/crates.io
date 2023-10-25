use http::{HeaderMap, HeaderValue};
use ipnetwork::IpNetwork;
use once_cell::sync::Lazy;
use std::iter::Iterator;
use std::net::IpAddr;
use std::str::from_utf8;

const X_FORWARDED_FOR: &str = "X-Forwarded-For";

const CLOUD_FRONT_STRS: &[&str] = &[
    // CloudFront IP addresses from http://d7uri8nf7uskq.cloudfront.net/tools/list-cloudfront-ips
    // Last updated: 2022-03-26
    "3.10.17.128/25",
    "3.11.53.0/24",
    "3.35.130.128/25",
    "3.101.158.0/23",
    "3.128.93.0/24",
    "3.134.215.0/24",
    "3.231.2.0/25",
    "3.234.232.224/27",
    "3.236.48.0/23",
    "3.236.169.192/26",
    "13.32.0.0/15",
    "13.35.0.0/16",
    "13.48.32.0/24",
    "13.54.63.128/26",
    "13.59.250.0/26",
    "13.113.196.64/26",
    "13.113.203.0/24",
    "13.124.199.0/24",
    "13.210.67.128/26",
    "13.224.0.0/14",
    "13.228.69.0/24",
    "13.233.177.192/26",
    "13.249.0.0/16",
    "15.158.0.0/16",
    "15.188.184.0/24",
    "15.207.13.128/25",
    "15.207.213.128/25",
    "18.64.0.0/14",
    "18.154.0.0/15",
    "18.160.0.0/15",
    "18.164.0.0/15",
    "18.172.0.0/15",
    "18.192.142.0/23",
    "18.200.212.0/23",
    "18.216.170.128/25",
    "18.229.220.192/26",
    "18.238.0.0/15",
    "18.244.0.0/15",
    "34.195.252.0/24",
    "34.216.51.0/25",
    "34.223.12.224/27",
    "34.223.80.192/26",
    "34.226.14.0/24",
    "35.158.136.0/24",
    "35.162.63.192/26",
    "35.167.191.128/26",
    "36.103.232.0/25",
    "36.103.232.128/26",
    "44.227.178.0/24",
    "44.234.90.252/30",
    "44.234.108.128/25",
    "52.15.127.128/26",
    "52.46.0.0/18",
    "52.47.139.0/24",
    "52.52.191.128/26",
    "52.56.127.0/25",
    "52.57.254.0/24",
    "52.66.194.128/26",
    "52.78.247.128/26",
    "52.82.128.0/19",
    "52.84.0.0/15",
    "52.124.128.0/17",
    "52.199.127.192/26",
    "52.212.248.0/26",
    "52.220.191.0/26",
    "52.222.128.0/17",
    "54.182.0.0/16",
    "54.192.0.0/16",
    "54.230.0.0/17",
    "54.230.128.0/18",
    "54.230.200.0/21",
    "54.230.208.0/20",
    "54.230.224.0/19",
    "54.233.255.128/26",
    "54.239.128.0/18",
    "54.239.192.0/19",
    "54.240.128.0/18",
    "58.254.138.0/25",
    "58.254.138.128/26",
    "64.252.64.0/18",
    "64.252.128.0/18",
    "65.8.0.0/16",
    "65.9.0.0/17",
    "65.9.128.0/18",
    "70.132.0.0/18",
    "71.152.0.0/17",
    "99.79.169.0/24",
    "99.84.0.0/16",
    "99.86.0.0/16",
    "108.138.0.0/15",
    "108.156.0.0/14",
    "116.129.226.0/25",
    "116.129.226.128/26",
    "118.193.97.64/26",
    "118.193.97.128/25",
    "119.147.182.0/25",
    "119.147.182.128/26",
    "120.52.12.64/26",
    "120.52.22.96/27",
    "120.52.39.128/27",
    "120.52.153.192/26",
    "120.232.236.0/25",
    "120.232.236.128/26",
    "120.253.240.192/26",
    "120.253.241.160/27",
    "120.253.245.128/26",
    "120.253.245.192/27",
    "130.176.0.0/17",
    "130.176.128.0/18",
    "130.176.192.0/19",
    "130.176.224.0/20",
    "143.204.0.0/16",
    "144.220.0.0/16",
    "180.163.57.0/25",
    "180.163.57.128/26",
    "204.246.164.0/22",
    "204.246.168.0/22",
    "204.246.172.0/24",
    "204.246.173.0/24",
    "204.246.174.0/23",
    "204.246.176.0/20",
    "205.251.200.0/21",
    "205.251.208.0/20",
    "205.251.249.0/24",
    "205.251.250.0/23",
    "205.251.252.0/23",
    "205.251.254.0/24",
    "216.137.32.0/19",
    "223.71.11.0/27",
    "223.71.71.96/27",
    "223.71.71.128/25",
];

static CLOUD_FRONT_NETWORKS: Lazy<Vec<IpNetwork>> = Lazy::new(|| {
    CLOUD_FRONT_STRS
        .iter()
        .map(|s| s.parse().unwrap())
        .collect()
});

fn is_cloud_front_ip(ip: &IpAddr) -> bool {
    CLOUD_FRONT_NETWORKS
        .iter()
        .any(|trusted_proxy| trusted_proxy.contains(*ip))
}

#[allow(dead_code)]
pub fn process_xff_headers(headers: &HeaderMap) -> Option<IpAddr> {
    let mut xff_iter = headers.get_all(X_FORWARDED_FOR).iter();
    let first_header = xff_iter.next()?;

    let has_more_headers = xff_iter.next().is_some();
    return if has_more_headers {
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

        parse_xff_header(first_header)
            .into_iter()
            .filter_map(|r| r.ok())
            .filter(|ip| !is_cloud_front_ip(ip))
            .next_back()
    };
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
