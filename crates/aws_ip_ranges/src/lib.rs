include!(concat!(env!("OUT_DIR"), "/types.rs"));

/// The content of <https://ip-ranges.amazonaws.com/ip-ranges.json> as a const [IpRanges] struct.
pub const IP_RANGES: IpRanges = include!(concat!(env!("OUT_DIR"), "/data.rs"));
