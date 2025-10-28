use std::error::Error;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use types::*;

mod types {
    include!("build_types.rs");
}

fn main() -> Result<(), Box<dyn Error>> {
    generate_types_module()?;
    generate_data_rs()?;
    Ok(())
}

fn generate_types_module() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=build_types.rs");

    let path = Path::new(&std::env::var("OUT_DIR")?).join("types.rs");
    let mut file = BufWriter::new(File::create(path)?);

    let build_types = include_str!("build_types.rs");
    let types = build_types
        .replace("use serde::Deserialize;\n", "")
        .replace("#[derive(", "#[derive(Debug, ")
        .replace(", Deserialize", "")
        .replace("#[serde(deny_unknown_fields)]\n", "")
        .replace("#[serde(rename = \"syncToken\")]\n", "")
        .replace("#[serde(rename = \"createDate\")]\n", "")
        .replace("String", "&'static str")
        .replace("Vec<", "&'static [")
        .replace(">,", "],");

    file.write_all(types.as_bytes())?;

    Ok(())
}

fn generate_data_rs() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=data/");

    let path = Path::new(&std::env::var("OUT_DIR")?).join("data.rs");
    let mut file = BufWriter::new(File::create(path)?);

    let ip_ranges = include_bytes!("data/ip-ranges.json");
    let ip_ranges: IpRanges = serde_json::from_slice(ip_ranges)?;
    writeln!(file, "{ip_ranges:?}")?;

    Ok(())
}

struct Array<'a, T> {
    array: &'a Vec<T>,
    indent: usize,
}

impl<'a, T> Array<'a, T> {
    fn new(array: &'a Vec<T>, indent: usize) -> Self {
        Self { array, indent }
    }
}

impl<T: Debug> Debug for Array<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "&[")?;
        for value in self.array {
            write!(f, "{}", "    ".repeat(self.indent + 1))?;
            writeln!(f, "{value:?},")?;
        }
        write!(f, "{}]", "    ".repeat(self.indent))
    }
}

impl Debug for IpRanges {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "IpRanges {{")?;
        writeln!(f, "    sync_token: {:?},", self.sync_token)?;
        writeln!(f, "    create_date: {:?},", self.create_date)?;
        writeln!(f, "    prefixes: {:?},", Array::new(&self.prefixes, 1))?;
        writeln!(
            f,
            "    ipv6_prefixes: {:?},",
            Array::new(&self.ipv6_prefixes, 1)
        )?;
        write!(f, "}}")
    }
}

impl Debug for Prefix {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Prefix {{")?;
        writeln!(f, "            ip_prefix: {:?},", self.ip_prefix)?;
        writeln!(f, "            region: {:?},", self.region)?;
        writeln!(f, "            service: {:?},", self.service)?;
        writeln!(
            f,
            "            network_border_group: {:?},",
            self.network_border_group
        )?;
        write!(f, "        }}")
    }
}

impl Debug for Ipv6Prefix {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Ipv6Prefix {{")?;
        writeln!(f, "            ipv6_prefix: {:?},", self.ipv6_prefix)?;
        writeln!(f, "            region: {:?},", self.region)?;
        writeln!(f, "            service: {:?},", self.service)?;
        writeln!(
            f,
            "            network_border_group: {:?},",
            self.network_border_group
        )?;
        write!(f, "        }}")
    }
}
