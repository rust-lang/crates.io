use chrono::NaiveDate;
use derive_deref::Deref;
use semver::Version;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

#[derive(Clone, Default, Deref)]
pub struct DownloadsMap(HashMap<(String, Version, NaiveDate), u64>);

impl DownloadsMap {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Increments the download count for the given crate version on the given date.
    pub fn add(&mut self, name: String, version: Version, date: NaiveDate) {
        *self.0.entry((name, version, date)).or_default() += 1;
    }

    /// Returns a [HashSet] of all crate names in the map.
    pub fn unique_crates(&self) -> HashSet<&str> {
        self.0.keys().map(|(krate, _, _)| krate.as_str()).collect()
    }

    /// Returns the total number of downloads across all crates and versions.
    pub fn sum_downloads(&self) -> u64 {
        self.0.values().sum()
    }

    /// Converts the map into a vector of `(crate, version, date, downloads)` tuples.
    pub fn into_vec(self) -> Vec<(String, Version, NaiveDate, u64)> {
        self.0
            .into_iter()
            .map(|((name, version, date), downloads)| (name, version, date, downloads))
            .collect()
    }
}

impl Debug for DownloadsMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut downloads = self
            .0
            .iter()
            .map(|((krate, version, date), downloads)| (date, krate, version, downloads))
            .collect::<Vec<_>>();

        downloads.sort();

        f.write_str("DownloadsMap {\n")?;
        for (date, krate, version, downloads) in downloads {
            f.write_str("    ")?;
            f.write_fmt(format_args!("{date}  {krate}@{version} .. {downloads}"))?;
            f.write_str("\n")?;
        }
        f.write_str("}")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use insta::assert_debug_snapshot;
    use semver::Version;

    fn add(downloads: &mut DownloadsMap, name: &str, version: &str, date: &str) {
        downloads.add(
            name.to_string(),
            version.parse::<Version>().unwrap(),
            date.parse::<NaiveDate>().unwrap(),
        );
    }

    #[test]
    fn test_downloads_map() {
        let mut downloads = DownloadsMap::new();

        // Add an entry to the map
        add(&mut downloads, "xmas", "2.0.0", "2023-12-25");
        assert_debug_snapshot!(downloads, @r"
        DownloadsMap {
            2023-12-25  xmas@2.0.0 .. 1
        }
        ");

        // Add the same entry again
        add(&mut downloads, "xmas", "2.0.0", "2023-12-25");
        assert_debug_snapshot!(downloads, @r"
        DownloadsMap {
            2023-12-25  xmas@2.0.0 .. 2
        }
        ");

        // Add other entries
        add(&mut downloads, "foo", "2.0.0", "2023-12-25");
        add(&mut downloads, "xmas", "1.0.0", "2023-12-25");
        add(&mut downloads, "xmas", "2.0.0", "2023-12-26");
        assert_debug_snapshot!(downloads, @r"
        DownloadsMap {
            2023-12-25  foo@2.0.0 .. 1
            2023-12-25  xmas@1.0.0 .. 1
            2023-12-25  xmas@2.0.0 .. 2
            2023-12-26  xmas@2.0.0 .. 1
        }
        ");
    }
}
