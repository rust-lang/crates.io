// TODO: Move all encodable types here
// For now, just reexport

use chrono::NaiveDateTime;

pub use badge::EncodableBadge;

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
