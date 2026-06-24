use std::collections::HashSet;
use std::net::IpAddr;
use std::str::FromStr;

use crates_io_env_vars::{list, list_parsed};
use tracing::warn;

use crate::middleware::block_traffic::BlockCriteria;

#[derive(Debug, Default)]
pub struct BlockConfig {
    /// Header values to block, keyed by header name.
    ///
    /// Read from the `BLOCKED_TRAFFIC` environment variable. See the
    /// [`block_traffic`](crate::middleware::block_traffic) middleware for more
    /// documentation.
    pub traffic: Vec<(String, Vec<BlockCriteria>)>,

    /// IP addresses that are blocked from accessing the API.
    ///
    /// Read from the `BLOCKED_IPS` environment variable.
    pub ips: HashSet<IpAddr>,

    /// HTTP route patterns that are manually blocked by an operator (e.g.
    /// `/crates/{crate_id}/{version}/download`).
    ///
    /// Read from the `BLOCKED_ROUTES` environment variable.
    pub routes: HashSet<String>,
}

impl BlockConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let traffic = blocked_traffic();
        let ips = HashSet::from_iter(list_parsed("BLOCKED_IPS", IpAddr::from_str)?);
        let routes = HashSet::from_iter(list("BLOCKED_ROUTES")?);

        Ok(Self {
            traffic,
            ips,
            routes,
        })
    }
}

fn blocked_traffic() -> Vec<(String, Vec<BlockCriteria>)> {
    let pattern_list = dotenvy::var("BLOCKED_TRAFFIC").unwrap_or_default();
    parse_traffic_patterns(&pattern_list)
        .map(|(header, value_env_var)| {
            let value_list = dotenvy::var(value_env_var).unwrap_or_default();
            let values = parse_traffic_pattern_values(header, &value_list);
            (header.into(), values)
        })
        .collect()
}

/// Extracts from the `BLOCKED_TRAFFIC` env var value a comma-separated list of pairs containing a
/// header name, an equals sign, and the name of another environment variable that contains the
/// values of that header that should be blocked. For example, if `BLOCKED_TRAFFIC` is set to
/// `User-Agent=BLOCKED_UAS,custom-header=BLOCKED_CUSTOM`, this function will return the pairs
/// (`User-Agent`, `BLOCKED_UAS`) and (`custom-header`, `BLOCKED_CUSTOM`).
///
/// Patterns that do not contain an `=` are skipped with a warning, so that a single
/// misconfigured entry does not prevent the remaining patterns from taking effect.
fn parse_traffic_patterns(patterns: &str) -> impl Iterator<Item = (&str, &str)> {
    patterns.split_terminator(',').filter_map(|pattern| {
        pattern.split_once('=').or_else(|| {
            warn!("Skipping invalid BLOCKED_TRAFFIC pattern `{pattern}`: expected HEADER=VALUE_ENV_VAR");
            None
        })
    })
}

/// After reading the value of an environment variable whose name was specified in the value of
/// `BLOCKED_TRAFFIC`, parses a comma-separated list of values to be used as either regex matches or
/// full string equality with the values of the header name specified in the `BLOCKED_TRAFFIC` pair.
///
/// Values that fail to parse are skipped with a warning, so that a single misconfigured entry
/// does not prevent the remaining values from taking effect.
fn parse_traffic_pattern_values(header: &str, value_list: &str) -> Vec<BlockCriteria> {
    value_list
        .split(',')
        .filter_map(|value| match value.try_into() {
            Ok(criteria) => Some(criteria),
            Err(error) => {
                warn!(
                    "Skipping invalid BLOCKED_TRAFFIC value `{value}` for header `{header}`: {error}"
                );
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_none;

    #[test]
    fn parse_traffic_patterns_splits_on_comma_and_looks_for_equal_sign() {
        let pattern_string_1 = "Foo=BAR,Bar=BAZ";
        let pattern_string_2 = "Baz=QUX";
        let pattern_string_3 = "";

        let patterns_1 = parse_traffic_patterns(pattern_string_1).collect::<Vec<_>>();
        assert_eq!(vec![("Foo", "BAR"), ("Bar", "BAZ")], patterns_1);

        let patterns_2 = parse_traffic_patterns(pattern_string_2).collect::<Vec<_>>();
        assert_eq!(vec![("Baz", "QUX")], patterns_2);

        assert_none!(parse_traffic_patterns(pattern_string_3).next());
    }

    #[test]
    fn parse_traffic_patterns_skips_entries_missing_equals_sign() {
        let pattern_string = "Foo=BAR,no-equals,Baz=QUX";

        let patterns = parse_traffic_patterns(pattern_string).collect::<Vec<_>>();
        assert_eq!(vec![("Foo", "BAR"), ("Baz", "QUX")], patterns);
    }

    #[test]
    fn parse_traffic_pattern_values_splits_on_comma_even_if_escaping_is_attempted() {
        let pattern = "web-tool 1.2.3,fancy-crate\\, run by fancy-author v4.5.6,/.*foo.*/";

        let values = parse_traffic_pattern_values("User-Agent", pattern);
        assert_eq!(
            vec![
                "web-tool 1.2.3",
                "fancy-crate\\",
                " run by fancy-author v4.5.6",
                ".*foo.*",
            ],
            values.iter().map(|r| r.as_str()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn parse_traffic_pattern_values_skips_invalid_regexes() {
        let pattern = "/valid-regex/,/[invalid-regex/,exact-string";

        let values = parse_traffic_pattern_values("User-Agent", pattern);
        assert_eq!(
            vec!["valid-regex", "exact-string"],
            values.iter().map(|r| r.as_str()).collect::<Vec<_>>()
        );
    }
}
