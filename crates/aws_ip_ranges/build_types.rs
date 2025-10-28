use serde::Deserialize;

/// The root object of <https://ip-ranges.amazonaws.com/ip-ranges.json>.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IpRanges {
    #[serde(rename = "syncToken")]
    pub sync_token: String,
    #[serde(rename = "createDate")]
    pub create_date: String,
    pub prefixes: Vec<Prefix>,
    pub ipv6_prefixes: Vec<Ipv6Prefix>,
}

/// The objects inside the `prefixes` list of
/// <https://ip-ranges.amazonaws.com/ip-ranges.json>.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prefix {
    pub ip_prefix: String,
    pub region: String,
    pub service: String,
    pub network_border_group: String,
}

/// The objects inside the `ipv6_prefixes` list of
/// <https://ip-ranges.amazonaws.com/ip-ranges.json>.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ipv6Prefix {
    pub ipv6_prefix: String,
    pub region: String,
    pub service: String,
    pub network_border_group: String,
}
