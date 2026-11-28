use chrono::{DateTime, Utc};
use crates_io_database::models::release_tracks::{ReleaseTrack, ReleaseTrackName};
use indexmap::IndexMap;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ReleaseTracks(IndexMap<ReleaseTrackName, ReleaseTrackDetails>);

/// Sorts the release tracks in descending order.
impl From<BTreeMap<ReleaseTrackName, ReleaseTrack<'_>>> for ReleaseTracks {
    fn from(tracks: BTreeMap<ReleaseTrackName, ReleaseTrack<'_>>) -> Self {
        let tracks = tracks.into_iter().rev();
        Self(tracks.map(|(name, track)| (name, track.into())).collect())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, utoipa::ToSchema)]
pub struct ReleaseTrackDetails {
    #[schema(value_type = String)]
    pub highest: semver::Version,

    /// Whether all versions in this release track are yanked.
    pub yanked: bool,

    /// The number of versions in this release track, including yanked
    /// versions and pre-releases.
    #[schema(example = 42, minimum = 1)]
    pub count: u64,

    /// When the first version in this release track was published.
    #[schema(example = "2019-12-13T13:46:41Z")]
    pub first_published: DateTime<Utc>,

    /// When the most recent version in this release track was published.
    #[schema(example = "2026-07-20T17:06:09Z")]
    pub last_published: DateTime<Utc>,
}

impl From<ReleaseTrack<'_>> for ReleaseTrackDetails {
    fn from(track: ReleaseTrack<'_>) -> Self {
        Self {
            highest: track.highest.num.clone(),
            // Non-yanked versions take precedence, so the highest version is
            // only yanked if all versions of the track are.
            yanked: track.highest.yanked,
            count: track.count,
            first_published: track.first_published,
            last_published: track.last_published,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ReleaseTracks;
    use chrono::{DateTime, TimeZone, Utc};
    use crates_io_database::models::default_versions::Version;
    use crates_io_database::models::release_tracks::{ReleaseTrackInput, calculate_release_tracks};
    use insta::assert_json_snapshot;

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
        let version = Version { id: 0, num, yanked };
        let created_at = published(day);
        ReleaseTrackInput {
            version,
            created_at,
        }
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

        let tracks = ReleaseTracks::from(calculate_release_tracks(&versions));
        assert_json_snapshot!(tracks, @r#"
        {
          "100": {
            "highest": "100.1.1",
            "yanked": false,
            "count": 2,
            "first_published": "2020-01-18T00:00:00Z",
            "last_published": "2020-01-19T00:00:00Z"
          },
          "1": {
            "highest": "1.3.5",
            "yanked": false,
            "count": 4,
            "first_published": "2020-01-14T00:00:00Z",
            "last_published": "2020-01-17T00:00:00Z"
          },
          "0.4": {
            "highest": "0.4.0-rc.1",
            "yanked": false,
            "count": 1,
            "first_published": "2020-01-13T00:00:00Z",
            "last_published": "2020-01-13T00:00:00Z"
          },
          "0.3": {
            "highest": "0.3.23",
            "yanked": false,
            "count": 8,
            "first_published": "2020-01-05T00:00:00Z",
            "last_published": "2020-01-12T00:00:00Z"
          },
          "0.2": {
            "highest": "0.2.1",
            "yanked": false,
            "count": 2,
            "first_published": "2020-01-03T00:00:00Z",
            "last_published": "2020-01-04T00:00:00Z"
          },
          "0.1": {
            "highest": "0.1.2",
            "yanked": false,
            "count": 2,
            "first_published": "2020-01-01T00:00:00Z",
            "last_published": "2020-01-02T00:00:00Z"
          }
        }
        "#);
    }
}
