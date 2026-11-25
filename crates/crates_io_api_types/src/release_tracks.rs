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
    use super::{ReleaseTrackDetails, ReleaseTrackName, ReleaseTracks, Version};
    use indexmap::IndexMap;
    use serde_json::json;

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
    fn release_tracks_empty() {
        assert_eq!(
            ReleaseTracks::from_versions(&[]),
            ReleaseTracks(IndexMap::new())
        );
    }

    #[test]
    fn release_tracks_prerelease() {
        let versions = [v("1.0.0-beta.5", false)];
        assert_eq!(
            ReleaseTracks::from_versions(&versions),
            ReleaseTracks(IndexMap::from([(
                ReleaseTrackName::Major(1),
                ReleaseTrackDetails {
                    highest: version("1.0.0-beta.5")
                }
            )]))
        );
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
        assert_eq!(
            ReleaseTracks::from_versions(&versions),
            ReleaseTracks(IndexMap::from([
                (
                    ReleaseTrackName::Major(3),
                    ReleaseTrackDetails {
                        highest: version("3.0.0-rc.1")
                    }
                ),
                (
                    ReleaseTrackName::Major(2),
                    ReleaseTrackDetails {
                        highest: version("2.0.0")
                    }
                ),
                (
                    ReleaseTrackName::Major(1),
                    ReleaseTrackDetails {
                        highest: version("1.1.0")
                    }
                ),
            ]))
        );
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

        let release_tracks = ReleaseTracks::from_versions(&versions);
        assert_eq!(
            release_tracks,
            ReleaseTracks(IndexMap::from([
                (
                    ReleaseTrackName::Major(100),
                    ReleaseTrackDetails {
                        highest: version("100.1.1")
                    }
                ),
                (
                    ReleaseTrackName::Major(1),
                    ReleaseTrackDetails {
                        highest: version("1.3.5")
                    }
                ),
                (
                    ReleaseTrackName::Minor(4),
                    ReleaseTrackDetails {
                        highest: version("0.4.0-rc.1")
                    }
                ),
                (
                    ReleaseTrackName::Minor(3),
                    ReleaseTrackDetails {
                        highest: version("0.3.23")
                    }
                ),
                (
                    ReleaseTrackName::Minor(2),
                    ReleaseTrackDetails {
                        highest: version("0.2.1")
                    }
                ),
                (
                    ReleaseTrackName::Minor(1),
                    ReleaseTrackDetails {
                        highest: version("0.1.2")
                    }
                ),
            ]))
        );

        let json = serde_json::from_str::<serde_json::Value>(
            &serde_json::to_string(&release_tracks).unwrap(),
        )
        .unwrap();
        assert_eq!(
            json,
            json!({
                "100": { "highest": "100.1.1" },
                "1": { "highest": "1.3.5" },
                "0.4": { "highest": "0.4.0-rc.1" },
                "0.3": { "highest": "0.3.23" },
                "0.2": { "highest": "0.2.1" },
                "0.1": { "highest": "0.1.2" }
            })
        );
    }
}
