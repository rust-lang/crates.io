use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use ipnetwork::{Ipv4Network, Ipv6Network};
use serde::Deserialize;

#[derive(Deserialize)]
struct IpRanges {
    prefixes: Vec<Prefix>,
    ipv6_prefixes: Vec<Ipv6Prefix>,
}

#[derive(Deserialize)]
struct Prefix {
    ip_prefix: String,
    service: String,
}

#[derive(Deserialize)]
struct Ipv6Prefix {
    ipv6_prefix: String,
    service: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=data/ip-ranges.json");

    let ip_ranges = include_bytes!("data/ip-ranges.json");
    let ip_ranges: IpRanges = serde_json::from_slice(ip_ranges)?;

    let path = Path::new(&std::env::var("OUT_DIR")?).join("cloudfront_networks.rs");
    let mut file = BufWriter::new(File::create(path)?);

    writeln!(
        file,
        "use ipnetwork::{{IpNetwork, Ipv4Network, Ipv6Network}};"
    )?;
    writeln!(file, "use std::net::{{Ipv4Addr, Ipv6Addr}};")?;
    writeln!(file)?;
    writeln!(file, "/// CloudFront IP ranges from AWS.")?;
    writeln!(file, "pub const CLOUDFRONT_NETWORKS: &[IpNetwork] = &[")?;

    for prefix in &ip_ranges.prefixes {
        if prefix.service == "CLOUDFRONT" {
            let network: Ipv4Network = prefix.ip_prefix.parse()?;
            let ip = network.ip();
            let prefix_len = network.prefix();

            writeln!(
                file,
                "    IpNetwork::V4(Ipv4Network::new_checked(Ipv4Addr::new({}, {}, {}, {}), {}).unwrap()),",
                ip.octets()[0],
                ip.octets()[1],
                ip.octets()[2],
                ip.octets()[3],
                prefix_len
            )?;
        }
    }

    for prefix in &ip_ranges.ipv6_prefixes {
        if prefix.service == "CLOUDFRONT" {
            let network: Ipv6Network = prefix.ipv6_prefix.parse()?;
            let ip = network.ip();
            let prefix_len = network.prefix();
            let segments = ip.segments();

            writeln!(
                file,
                "    IpNetwork::V6(Ipv6Network::new_checked(Ipv6Addr::new({:#x}, {:#x}, {:#x}, {:#x}, {:#x}, {:#x}, {:#x}, {:#x}), {}).unwrap()),",
                segments[0],
                segments[1],
                segments[2],
                segments[3],
                segments[4],
                segments[5],
                segments[6],
                segments[7],
                prefix_len
            )?;
        }
    }

    writeln!(file, "];")?;

    Ok(())
}
