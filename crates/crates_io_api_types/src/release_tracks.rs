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
}

impl From<ReleaseTrack<'_>> for ReleaseTrackDetails {
    fn from(track: ReleaseTrack<'_>) -> Self {
        Self {
            highest: track.highest.num.clone(),
            // Non-yanked versions take precedence, so the highest version is
            // only yanked if all versions of the track are.
            yanked: track.highest.yanked,
            count: track.count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ReleaseTracks;
    use crates_io_database::models::default_versions::Version;
    use crates_io_database::models::release_tracks::calculate_release_tracks;
    use insta::assert_json_snapshot;

    #[track_caller]
    fn version(str: &str) -> semver::Version {
        semver::Version::parse(str).unwrap()
    }

    #[track_caller]
    fn v(num: &str, yanked: bool) -> Version {
        let num = version(num);
        Version { id: 0, num, yanked }
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

        let tracks = ReleaseTracks::from(calculate_release_tracks(&versions));
        assert_json_snapshot!(tracks, @r#"
        {
          "100": {
            "highest": "100.1.1",
            "yanked": false,
            "count": 2
          },
          "1": {
            "highest": "1.3.5",
            "yanked": false,
            "count": 4
          },
          "0.4": {
            "highest": "0.4.0-rc.1",
            "yanked": false,
            "count": 1
          },
          "0.3": {
            "highest": "0.3.23",
            "yanked": false,
            "count": 8
          },
          "0.2": {
            "highest": "0.2.1",
            "yanked": false,
            "count": 2
          },
          "0.1": {
            "highest": "0.1.2",
            "yanked": false,
            "count": 2
          }
        }
        "#);
    }
}
