//! Edge-filtered log lines in the `{date}\t{name}\t{version}` format.

use crate::DownloadsMap;
use crate::date::parse_date;
use anyhow::Context;
use chrono::NaiveDate;
use foldhash::quality::FixedState;
use semver::Version;
use std::collections::HashMap;

/// Counts identical lines first, so that each distinct line is only split
/// and parsed once.
#[derive(Default)]
pub struct LeanLines(HashMap<Box<str>, u64, FixedState>);

impl LeanLines {
    pub fn add(&mut self, line: &str) {
        match self.0.get_mut(line) {
            Some(count) => *count += 1,
            None => _ = self.0.insert(line.into(), 1),
        }
    }

    pub fn add_to(self, downloads: &mut DownloadsMap) -> anyhow::Result<()> {
        for (line, count) in self.0 {
            let (date, name, version) =
                parse(&line).with_context(|| format!("Failed to parse log line: {line:?}"))?;
            downloads.add_count(name.to_string(), version, date, count);
        }
        Ok(())
    }
}

fn parse(line: &str) -> anyhow::Result<(NaiveDate, &str, Version)> {
    let (date, rest) = line.split_once('\t').context("Missing crate name")?;
    let (name, version) = rest.split_once('\t').context("Missing version")?;
    anyhow::ensure!(!name.is_empty(), "Missing crate name");
    let date = parse_date(date).context("Invalid date")?;
    let version = Version::parse(version).context("Invalid version")?;
    Ok((date, name, version))
}
