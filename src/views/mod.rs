use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use chrono::{NaiveDate, NaiveDateTime};
use semver;
use serde::{self, Deserialize, Deserializer, Serialize, Serializer};

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct EncodableBadge {
    pub badge_type: String,
    pub attributes: HashMap<String, Option<String>>,
}

use util::errors::{human, CargoError, CargoResult};

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodableCategory {
    pub id: String,
    pub category: String,
    pub slug: String,
    pub description: String,
    #[serde(with = "::util::rfc3339")]
    pub created_at: NaiveDateTime,
    pub crates_cnt: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodableCategoryWithSubcategories {
    pub id: String,
    pub category: String,
    pub slug: String,
    pub description: String,
    #[serde(with = "::util::rfc3339")]
    pub created_at: NaiveDateTime,
    pub crates_cnt: i32,
    pub subcategories: Vec<EncodableCategory>,
}

/// The serialization format for the `CrateOwnerInvitation` model.
#[derive(Deserialize, Serialize, Debug)]
pub struct EncodableCrateOwnerInvitation {
    pub invited_by_username: String,
    pub crate_name: String,
    pub crate_id: i32,
    #[serde(with = "::util::rfc3339")]
    pub created_at: NaiveDateTime,
}

#[derive(Deserialize, Serialize, Debug, Copy, Clone)]
pub struct InvitationResponse {
    pub crate_id: i32,
    pub accepted: bool,
}

pub use dependency::EncodableDependency;
pub use download::EncodableVersionDownload;

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodableKeyword {
    pub id: String,
    pub keyword: String,
    #[serde(with = "::util::rfc3339")]
    pub created_at: NaiveDateTime,
    pub crates_cnt: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodableCrate {
    pub id: String,
    pub name: String,
    #[serde(with = "::util::rfc3339")]
    pub updated_at: NaiveDateTime,
    pub versions: Option<Vec<i32>>,
    pub keywords: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub badges: Option<Vec<EncodableBadge>>,
    #[serde(with = "::util::rfc3339")]
    pub created_at: NaiveDateTime,
    pub downloads: i32,
    pub recent_downloads: Option<i64>,
    pub max_version: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub repository: Option<String>,
    pub links: EncodableCrateLinks,
    pub exact_match: bool,
    pub max_build_info_stable: Option<String>,
    pub max_build_info_beta: Option<String>,
    pub max_build_info_nightly: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodableCrateLinks {
    pub version_downloads: String,
    pub versions: Option<String>,
    pub owners: Option<String>,
    pub owner_team: Option<String>,
    pub owner_user: Option<String>,
    pub reverse_dependencies: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodableOwner {
    pub id: i32,
    pub login: String,
    pub kind: String,
    pub url: Option<String>,
    pub name: Option<String>,
    pub avatar: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct EncodableTeam {
    pub id: i32,
    pub login: String,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub url: Option<String>,
}

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

/// `MaxBuildInfo` in JSON form.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct EncodableMaxVersionBuildInfo {
    pub stable: Option<String>,
    pub beta: Option<String>,
    pub nightly: Option<String>,
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

impl ParsedRustChannelVersion {
    pub fn as_stable(&self) -> Option<&semver::Version> {
        match *self {
            ParsedRustChannelVersion::Stable(ref v) => Some(v),
            _ => None,
        }
    }

    pub fn as_beta(&self) -> Option<&NaiveDate> {
        match *self {
            ParsedRustChannelVersion::Beta(ref v) => Some(v),
            _ => None,
        }
    }

    pub fn as_nightly(&self) -> Option<&NaiveDate> {
        match *self {
            ParsedRustChannelVersion::Nightly(ref v) => Some(v),
            _ => None,
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
                build_info: "".to_string(),
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

    #[test]
    fn crate_serializes_to_rfc3399() {
        let crt = EncodableCrate {
            id: "".to_string(),
            name: "".to_string(),
            updated_at: NaiveDate::from_ymd(2017, 1, 6).and_hms(14, 23, 11),
            versions: None,
            keywords: None,
            categories: None,
            badges: None,
            created_at: NaiveDate::from_ymd(2017, 1, 6).and_hms(14, 23, 12),
            downloads: 0,
            recent_downloads: None,
            max_version: "".to_string(),
            max_build_info_stable: None,
            max_build_info_beta: None,
            max_build_info_nightly: None,
            description: None,
            homepage: None,
            documentation: None,
            repository: None,
            links: EncodableCrateLinks {
                version_downloads: "".to_string(),
                versions: None,
                owners: None,
                owner_team: None,
                owner_user: None,
                reverse_dependencies: "".to_string(),
            },
            exact_match: false,
        };
        let json = serde_json::to_string(&crt).unwrap();
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

    #[test]
    fn crate_owner_invitation_serializes_to_rfc3339() {
        let inv = EncodableCrateOwnerInvitation {
            invited_by_username: "".to_string(),
            crate_name: "".to_string(),
            crate_id: 123,
            created_at: NaiveDate::from_ymd(2017, 1, 6).and_hms(14, 23, 11),
        };
        let json = serde_json::to_string(&inv).unwrap();
        assert!(
            json.as_str()
                .find(r#""created_at":"2017-01-06T14:23:11+00:00""#)
                .is_some()
        );
    }
}
