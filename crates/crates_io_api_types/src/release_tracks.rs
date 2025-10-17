use indexmap::IndexMap;
use serde::Serialize;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ReleaseTracks(IndexMap<ReleaseTrackName, ReleaseTrackDetails>);

impl ReleaseTracks {
    // Return the release tracks based on a sorted semver versions iterator (in descending order).
    // **Remember to** filter out yanked versions manually before calling this function.
    pub fn from_sorted_semver_iter<'a, I>(versions: I) -> Self
    where
        I: Iterator<Item = &'a semver::Version>,
    {
        let mut map = IndexMap::new();
        for num in versions.filter(|num| num.pre.is_empty()) {
            let key = ReleaseTrackName::from_semver(num);
            let prev = map.last();
            if prev.filter(|&(k, _)| *k == key).is_none() {
                map.insert(
                    key,
                    ReleaseTrackDetails {
                        highest: num.clone(),
                    },
                );
            }
        }

        Self(map)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ReleaseTrackName {
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

impl std::fmt::Display for ReleaseTrackName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Minor(minor) => write!(f, "0.{minor}"),
            Self::Major(major) => write!(f, "{major}"),
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

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ReleaseTrackDetails {
    pub highest: semver::Version,
}

#[cfg(test)]
mod tests {
    use super::{ReleaseTrackDetails, ReleaseTrackName, ReleaseTracks};
    use indexmap::IndexMap;
    use serde_json::json;

    #[track_caller]
    fn version(str: &str) -> semver::Version {
        semver::Version::parse(str).unwrap()
    }

    #[test]
    fn release_tracks_empty() {
        let versions = [];
        assert_eq!(
            ReleaseTracks::from_sorted_semver_iter(versions.into_iter()),
            ReleaseTracks(IndexMap::new())
        );
    }

    #[test]
    fn release_tracks_prerelease() {
        let versions = [version("1.0.0-beta.5")];
        assert_eq!(
            ReleaseTracks::from_sorted_semver_iter(versions.iter()),
            ReleaseTracks(IndexMap::new())
        );
    }

    #[test]
    fn release_tracks_multiple() {
        let versions = [
            "100.1.1",
            "100.1.0",
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
        .map(version);

        let release_tracks = ReleaseTracks::from_sorted_semver_iter(versions.iter());
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
                "0.3": { "highest": "0.3.23" },
                "0.2": { "highest": "0.2.1" },
                "0.1": { "highest": "0.1.2" }
            })
        );
    }
}
