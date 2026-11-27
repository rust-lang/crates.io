use crates_io_database::models::default_versions::Version;
use derive_more::Display;
use indexmap::IndexMap;
use serde::Serialize;
use std::cmp::max;
use std::collections::BTreeMap;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ReleaseTracks(IndexMap<ReleaseTrackName, ReleaseTrackDetails>);

impl ReleaseTracks {
    /// Returns the release tracks of the given versions, sorted in descending
    /// order.
    ///
    /// The highest version of a track is chosen like the default version of a
    /// crate, see [`update_default_version()`](crates_io_database::models::update_default_version).
    pub fn from_versions<'a>(versions: impl IntoIterator<Item = &'a Version>) -> Self {
        let mut tracks = BTreeMap::new();
        for version in versions {
            let key = ReleaseTrackName::from_semver(&version.num);
            tracks
                .entry(key)
                .and_modify(|highest| *highest = max(*highest, version))
                .or_insert(version);
        }

        let tracks = tracks.into_iter().rev().map(|(key, version)| {
            let highest = version.num.clone();
            (key, ReleaseTrackDetails { highest })
        });

        Self(tracks.collect())
    }
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

#[derive(Debug, Clone, Eq, PartialEq, Serialize, utoipa::ToSchema)]
pub struct ReleaseTrackDetails {
    #[schema(value_type = String)]
    pub highest: semver::Version,
}

#[cfg(test)]
mod tests {
    use super::{ReleaseTracks, Version};
    use insta::{assert_json_snapshot, assert_snapshot};

    #[track_caller]
    fn version(str: &str) -> semver::Version {
        semver::Version::parse(str).unwrap()
    }

    #[track_caller]
    fn v(num: &str, yanked: bool) -> Version {
        let num = version(num);
        Version { id: 0, num, yanked }
    }

    fn format_tracks(tracks: &ReleaseTracks) -> String {
        let lines = tracks.0.iter();
        let lines = lines.map(|(name, details)| format!("{name}: {}\n", details.highest));
        lines.collect()
    }

    #[test]
    fn release_tracks_empty() {
        let tracks = ReleaseTracks::from_versions(&[]);
        assert_snapshot!(format_tracks(&tracks), @"");
    }

    #[test]
    fn release_tracks_prerelease() {
        let versions = [v("1.0.0-beta.5", false)];
        let tracks = ReleaseTracks::from_versions(&versions);
        assert_snapshot!(format_tracks(&tracks), @"1: 1.0.0-beta.5");
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
        let tracks = ReleaseTracks::from_versions(&versions);
        assert_snapshot!(format_tracks(&tracks), @"
        3: 3.0.0-rc.1
        2: 2.0.0
        1: 1.1.0
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

        let tracks = ReleaseTracks::from_versions(&versions);
        assert_snapshot!(format_tracks(&tracks), @"
        100: 100.1.1
        1: 1.3.5
        0.4: 0.4.0-rc.1
        0.3: 0.3.23
        0.2: 0.2.1
        0.1: 0.1.2
        ");
        assert_json_snapshot!(tracks, @r#"
        {
          "100": {
            "highest": "100.1.1"
          },
          "1": {
            "highest": "1.3.5"
          },
          "0.4": {
            "highest": "0.4.0-rc.1"
          },
          "0.3": {
            "highest": "0.3.23"
          },
          "0.2": {
            "highest": "0.2.1"
          },
          "0.1": {
            "highest": "0.1.2"
          }
        }
        "#);
    }
}
