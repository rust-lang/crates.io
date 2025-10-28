use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

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

    let path = Path::new(&std::env::var("OUT_DIR")?).join("cloudfront_cidrs.rs");
    let mut file = BufWriter::new(File::create(path)?);

    writeln!(file, "/// CloudFront IP ranges from AWS.")?;
    writeln!(file, "pub const CLOUDFRONT_CIDRS: &[&str] = &[")?;

    for prefix in &ip_ranges.prefixes {
        if prefix.service == "CLOUDFRONT" {
            writeln!(file, "    {:?},", prefix.ip_prefix)?;
        }
    }

    for prefix in &ip_ranges.ipv6_prefixes {
        if prefix.service == "CLOUDFRONT" {
            writeln!(file, "    {:?},", prefix.ipv6_prefix)?;
        }
    }

    writeln!(file, "];")?;

    Ok(())
}
