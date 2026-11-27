use crate::models::default_versions;
use derive_more::Display;
use std::cmp::max;
use std::collections::BTreeMap;

/// Groups the given versions by release track, sorted in ascending order.
///
/// The highest version of a track is chosen like the default version of a
/// crate, see [`update_default_version()`](crate::models::update_default_version).
pub fn calculate_release_tracks<'a>(
    versions: impl IntoIterator<Item = &'a default_versions::Version>,
) -> BTreeMap<ReleaseTrackName, ReleaseTrack<'a>> {
    let mut tracks = BTreeMap::new();
    for version in versions {
        let key = ReleaseTrackName::from_semver(&version.num);
        tracks
            .entry(key)
            .and_modify(|track: &mut ReleaseTrack<'a>| track.add(version))
            .or_insert_with(|| ReleaseTrack::new(version));
    }
    tracks
}

#[derive(Debug, Clone, Display, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ReleaseTrackName {
    #[display("0.{_0}")]
    Minor(u64),
    Major(u64),
}

impl ReleaseTrackName {
    pub fn from_semver(version: &semver::Version) -> Self {
        if version.major == 0 {
            Self::Minor(version.minor)
        } else {
            Self::Major(version.major)
        }
    }
}

impl serde::Serialize for ReleaseTrackName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        Self: std::fmt::Display,
    {
        serializer.collect_str(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseTrack<'a> {
    pub highest: &'a default_versions::Version,
    /// The number of versions in this release track, including yanked
    /// versions and pre-releases.
    pub count: u64,
}

impl<'a> ReleaseTrack<'a> {
    fn new(version: &'a default_versions::Version) -> Self {
        Self {
            highest: version,
            count: 1,
        }
    }

    fn add(&mut self, version: &'a default_versions::Version) {
        self.highest = max(self.highest, version);
        self.count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    #[track_caller]
    fn version(str: &str) -> semver::Version {
        semver::Version::parse(str).unwrap()
    }

    #[track_caller]
    fn v(num: &str, yanked: bool) -> default_versions::Version {
        let num = version(num);
        default_versions::Version { id: 0, num, yanked }
    }

    fn format_tracks(tracks: &BTreeMap<ReleaseTrackName, ReleaseTrack<'_>>) -> String {
        let lines = tracks.iter().map(|(name, track)| {
            let ReleaseTrack { highest, count } = track;
            let num = &highest.num;
            let yanked = if highest.yanked { " (yanked)" } else { "" };
            format!("{name}: {num}{yanked}, count: {count}\n")
        });
        lines.collect()
    }

    #[test]
    fn release_tracks_empty() {
        let tracks = calculate_release_tracks(&[]);
        assert_snapshot!(format_tracks(&tracks), @"");
    }

    #[test]
    fn release_tracks_prerelease() {
        let versions = [v("1.0.0-beta.5", false)];
        let tracks = calculate_release_tracks(&versions);
        assert_snapshot!(format_tracks(&tracks), @"1: 1.0.0-beta.5, count: 1");
    }

    #[test]
    fn release_tracks_yanked() {
        let versions = [
            v("3.0.0", true),
            v("3.0.0-rc.1", false),
            v("2.1.0", true),
            v("2.0.0", false),
            v("1.1.0", true),
            v("1.1.0-beta.1", true),
            v("1.0.0", true),
        ];
        let tracks = calculate_release_tracks(&versions);
        assert_snapshot!(format_tracks(&tracks), @"
        1: 1.1.0 (yanked), count: 3
        2: 2.0.0, count: 2
        3: 3.0.0-rc.1, count: 2
        ");
    }

    #[test]
    fn release_tracks_multiple() {
        let versions = [
            "100.1.1",
            "100.1.0",
            "1.4.0-beta.1",
            "1.3.5",
            "1.2.5",
            "1.1.5",
            "0.4.0-rc.1",
            "0.3.23",
            "0.3.22",
            "0.3.21-pre.0",
            "0.3.20",
            "0.3.3",
            "0.3.2",
            "0.3.1",
            "0.3.0",
            "0.2.1",
            "0.2.0",
            "0.1.2",
            "0.1.1",
        ]
        .map(|num| v(num, false));

        let tracks = calculate_release_tracks(&versions);
        assert_snapshot!(format_tracks(&tracks), @"
        0.1: 0.1.2, count: 2
        0.2: 0.2.1, count: 2
        0.3: 0.3.23, count: 8
        0.4: 0.4.0-rc.1, count: 1
        1: 1.3.5, count: 4
        100: 100.1.1, count: 2
        ");
    }
}
