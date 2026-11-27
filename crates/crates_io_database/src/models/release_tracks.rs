use crate::models::default_versions;
use crate::schema::versions;
use chrono::{DateTime, Utc};
use derive_more::Display;
use diesel::prelude::*;
use std::cmp::{max, min};
use std::collections::BTreeMap;

/// The columns of the `versions` table that the release track calculation
/// needs.
#[derive(Clone, Debug, PartialEq, Eq, HasQuery)]
#[diesel(table_name = versions)]
pub struct ReleaseTrackInput {
    #[diesel(embed)]
    pub version: default_versions::Version,
    pub created_at: DateTime<Utc>,
}

/// Groups the given versions by release track, sorted in ascending order.
///
/// The highest version of a track is chosen like the default version of a
/// crate, see [`update_default_version()`](crate::models::update_default_version).
pub fn calculate_release_tracks<'a>(
    rows: impl IntoIterator<Item = &'a ReleaseTrackInput>,
) -> BTreeMap<ReleaseTrackName, ReleaseTrack<'a>> {
    let mut tracks = BTreeMap::new();
    for row in rows {
        let key = ReleaseTrackName::from_semver(&row.version.num);
        tracks
            .entry(key)
            .and_modify(|track: &mut ReleaseTrack<'a>| track.add(row))
            .or_insert_with(|| ReleaseTrack::new(row));
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
    /// When the first version in this release track was published.
    pub first_published: DateTime<Utc>,
    /// When the most recent version in this release track was published.
    pub last_published: DateTime<Utc>,
}

impl<'a> ReleaseTrack<'a> {
    fn new(row: &'a ReleaseTrackInput) -> Self {
        Self {
            highest: &row.version,
            count: 1,
            first_published: row.created_at,
            last_published: row.created_at,
        }
    }

    fn add(&mut self, row: &'a ReleaseTrackInput) {
        self.highest = max(self.highest, &row.version);
        self.count += 1;
        self.first_published = min(self.first_published, row.created_at);
        self.last_published = max(self.last_published, row.created_at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use insta::assert_snapshot;

    #[track_caller]
    fn version(str: &str) -> semver::Version {
        semver::Version::parse(str).unwrap()
    }

    fn published(day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2020, 1, day, 0, 0, 0).unwrap()
    }

    #[track_caller]
    fn v(num: &str, yanked: bool, day: u32) -> ReleaseTrackInput {
        let num = version(num);
        let version = default_versions::Version { id: 0, num, yanked };
        let created_at = published(day);
        ReleaseTrackInput {
            version,
            created_at,
        }
    }

    fn format_tracks(tracks: &BTreeMap<ReleaseTrackName, ReleaseTrack<'_>>) -> String {
        let lines = tracks.iter().map(|(name, track)| {
            let ReleaseTrack { highest, count, .. } = track;
            let num = &highest.num;
            let yanked = if highest.yanked { " (yanked)" } else { "" };
            let first = track.first_published.date_naive();
            let last = track.last_published.date_naive();
            format!("{name}: {num}{yanked}, count: {count}, published: {first}..{last}\n")
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
        let versions = [v("1.0.0-beta.5", false, 1)];
        let tracks = calculate_release_tracks(&versions);
        assert_snapshot!(format_tracks(&tracks), @"1: 1.0.0-beta.5, count: 1, published: 2020-01-01..2020-01-01");
    }

    #[test]
    fn release_tracks_yanked() {
        let versions = [
            v("3.0.0", true, 7),
            v("3.0.0-rc.1", false, 6),
            v("2.1.0", true, 5),
            v("2.0.0", false, 4),
            v("1.1.0", true, 3),
            v("1.1.0-beta.1", true, 2),
            v("1.0.0", true, 1),
        ];
        let tracks = calculate_release_tracks(&versions);
        assert_snapshot!(format_tracks(&tracks), @"
        1: 1.1.0 (yanked), count: 3, published: 2020-01-01..2020-01-03
        2: 2.0.0, count: 2, published: 2020-01-04..2020-01-05
        3: 3.0.0-rc.1, count: 2, published: 2020-01-06..2020-01-07
        ");
    }

    #[test]
    fn release_tracks_multiple() {
        let versions = [
            ("100.1.1", 19),
            ("100.1.0", 18),
            ("1.4.0-beta.1", 16),
            ("1.3.5", 15),
            ("1.2.5", 14),
            ("1.1.5", 17),
            ("0.4.0-rc.1", 13),
            ("0.3.23", 12),
            ("0.3.22", 11),
            ("0.3.21-pre.0", 10),
            ("0.3.20", 9),
            ("0.3.3", 8),
            ("0.3.2", 7),
            ("0.3.1", 6),
            ("0.3.0", 5),
            ("0.2.1", 4),
            ("0.2.0", 3),
            ("0.1.2", 2),
            ("0.1.1", 1),
        ]
        .map(|(num, day)| v(num, false, day));

        let tracks = calculate_release_tracks(&versions);
        assert_snapshot!(format_tracks(&tracks), @"
        0.1: 0.1.2, count: 2, published: 2020-01-01..2020-01-02
        0.2: 0.2.1, count: 2, published: 2020-01-03..2020-01-04
        0.3: 0.3.23, count: 8, published: 2020-01-05..2020-01-12
        0.4: 0.4.0-rc.1, count: 1, published: 2020-01-13..2020-01-13
        1: 1.3.5, count: 4, published: 2020-01-14..2020-01-17
        100: 100.1.1, count: 2, published: 2020-01-18..2020-01-19
        ");
    }
}
