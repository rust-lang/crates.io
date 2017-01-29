// TODO: Move all encodable types here
// For now, just reexport

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use chrono::{NaiveDate, NaiveDateTime};
use semver;
use serde::{self, Deserialize, Deserializer, Serialize, Serializer};

pub use badge::EncodableBadge;

use util::errors::{human, CargoError, CargoResult};

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodableCategory {
    pub id: String,
    pub category: String,
    pub slug: String,
    pub description: String,
    #[serde(with = "::util::rfc3339")] pub created_at: NaiveDateTime,
    pub crates_cnt: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodableCategoryWithSubcategories {
    pub id: String,
    pub category: String,
    pub slug: String,
    pub description: String,
    #[serde(with = "::util::rfc3339")] pub created_at: NaiveDateTime,
    pub crates_cnt: i32,
    pub subcategories: Vec<EncodableCategory>,
}

pub use crate_owner_invitation::{EncodableCrateOwnerInvitation, InvitationResponse};
pub use dependency::EncodableDependency;
pub use download::EncodableVersionDownload;

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodableKeyword {
    pub id: String,
    pub keyword: String,
    #[serde(with = "::util::rfc3339")] pub created_at: NaiveDateTime,
    pub crates_cnt: i32,
}

pub use krate::EncodableCrate;
pub use owner::{EncodableOwner, EncodableTeam};
pub use token::EncodableApiTokenWithToken;
pub use user::{EncodablePrivateUser, EncodablePublicUser};
pub use version::{EncodableVersion, EncodableVersionLinks};

// TODO: Prefix many of these with `Encodable` then clean up the reexports
pub mod krate_publish;
pub use self::krate_publish::CrateDependency as EncodableCrateDependency;
pub use self::krate_publish::NewCrate as EncodableCrateUpload;

/// Information about whether this version built on the specified Rust version and target, as
/// uploaded by the `cargo publish-build-info` command.
#[derive(Debug, Serialize, Deserialize)]
pub struct EncodableVersionBuildInfoUpload {
    pub rust_version: EncodableRustChannelVersion,
    pub target: String,
    pub passed: bool,
}

/// Aggregated build info for a crate version grouped by Rust channel for front-end display
/// convenience.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct EncodableVersionBuildInfo {
    pub id: i32,
    pub ordering: HashMap<String, Vec<String>>,
    pub stable: HashMap<String, HashMap<String, bool>>,
    pub beta: HashMap<NaiveDate, HashMap<String, bool>>,
    pub nightly: HashMap<NaiveDate, HashMap<String, bool>>,
}

/// Describes a Rust version by its channel and the released version on that channel.
/// For use in describing what versions of Rust a particular crate version builds with.
/// Contains the original version string for inserting into the database.
#[derive(Debug)]
pub struct EncodableRustChannelVersion {
    raw: String,
    pub parsed: ParsedRustChannelVersion,
}

/// A pretty, minimal representation of a Rust version's channel and released version on that
/// channel. Namely, does not include the exact release hash.
#[derive(Debug)]
pub enum ParsedRustChannelVersion {
    Stable(semver::Version),
    Beta(NaiveDate),
    Nightly(NaiveDate),
}

impl fmt::Display for EncodableRustChannelVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl Serialize for EncodableRustChannelVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.raw)
    }
}

impl<'de> Deserialize<'de> for EncodableRustChannelVersion {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<EncodableRustChannelVersion, D::Error> {
        let s = String::deserialize(d)?;

        Ok(EncodableRustChannelVersion {
            raw: s.clone(),
            parsed: ParsedRustChannelVersion::from_str(&s).map_err(serde::de::Error::custom)?,
        })
    }
}

impl FromStr for ParsedRustChannelVersion {
    type Err = Box<CargoError>;

    fn from_str(s: &str) -> CargoResult<Self> {
        // Recognized formats:
        // rustc 1.14.0 (e8a012324 2016-12-16)
        // rustc 1.15.0-beta.5 (10893a9a3 2017-01-19)
        // rustc 1.16.0-nightly (df8debf6d 2017-01-25)

        let pieces: Vec<_> = s.split(&[' ', '(', ')'][..])
            .filter(|s| !s.trim().is_empty())
            .collect();

        if pieces.len() != 4 {
            return Err(human(&format_args!(
                "rust_version `{}` not recognized; \
                 expected format like `rustc X.Y.Z (SHA YYYY-MM-DD)`",
                s
            )));
        }

        if pieces[1].contains("nightly") {
            Ok(ParsedRustChannelVersion::Nightly(
                NaiveDate::parse_from_str(pieces[3], "%Y-%m-%d")?,
            ))
        } else if pieces[1].contains("beta") {
            Ok(ParsedRustChannelVersion::Beta(NaiveDate::parse_from_str(
                pieces[3],
                "%Y-%m-%d",
            )?))
        } else {
            let v = semver::Version::parse(pieces[1])?;
            if v.pre.is_empty() {
                Ok(ParsedRustChannelVersion::Stable(v))
            } else {
                Err(human(&format_args!(
                    "rust_version `{}` not recognized as nightly, beta, or stable",
                    s
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use super::*;
    use chrono::NaiveDate;
    use serde_json;

    #[test]
    fn category_dates_serializes_to_rfc3339() {
        let cat = EncodableCategory {
            id: "".to_string(),
            category: "".to_string(),
            slug: "".to_string(),
            description: "".to_string(),
            crates_cnt: 1,
            created_at: NaiveDate::from_ymd(2017, 1, 6).and_hms(14, 23, 11),
        };
        let json = serde_json::to_string(&cat).unwrap();
        assert!(
            json.as_str()
                .find(r#""created_at":"2017-01-06T14:23:11+00:00""#)
                .is_some()
        );
    }

    #[test]
    fn category_with_sub_dates_serializes_to_rfc3339() {
        let cat = EncodableCategoryWithSubcategories {
            id: "".to_string(),
            category: "".to_string(),
            slug: "".to_string(),
            description: "".to_string(),
            crates_cnt: 1,
            created_at: NaiveDate::from_ymd(2017, 1, 6).and_hms(14, 23, 11),
            subcategories: vec![],
        };
        let json = serde_json::to_string(&cat).unwrap();
        assert!(
            json.as_str()
                .find(r#""created_at":"2017-01-06T14:23:11+00:00""#)
                .is_some()
        );
    }

    #[test]
    fn keyword_serializes_to_rfc3339() {
        let key = EncodableKeyword {
            id: "".to_string(),
            keyword: "".to_string(),
            created_at: NaiveDate::from_ymd(2017, 1, 6).and_hms(14, 23, 11),
            crates_cnt: 0,
        };
        let json = serde_json::to_string(&key).unwrap();
        assert!(
            json.as_str()
                .find(r#""created_at":"2017-01-06T14:23:11+00:00""#)
                .is_some()
        );
    }

    #[test]
    fn version_serializes_to_rfc3339() {
        let ver = EncodableVersion {
            id: 1,
            krate: "".to_string(),
            num: "".to_string(),
            dl_path: "".to_string(),
            readme_path: "".to_string(),
            updated_at: NaiveDate::from_ymd(2017, 1, 6).and_hms(14, 23, 11),
            created_at: NaiveDate::from_ymd(2017, 1, 6).and_hms(14, 23, 12),
            downloads: 0,
            features: HashMap::new(),
            yanked: false,
            license: None,
            links: EncodableVersionLinks {
                dependencies: "".to_string(),
                version_downloads: "".to_string(),
                authors: "".to_string(),
            },
        };
        let json = serde_json::to_string(&ver).unwrap();
        assert!(
            json.as_str()
                .find(r#""updated_at":"2017-01-06T14:23:11+00:00""#)
                .is_some()
        );
        assert!(
            json.as_str()
                .find(r#""created_at":"2017-01-06T14:23:12+00:00""#)
                .is_some()
        );
    }
}
