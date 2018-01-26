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
pub use version::EncodableVersion;

// TODO: Prefix many of these with `Encodable` then clean up the reexports
pub mod krate_publish;
pub use self::krate_publish::CrateDependency as EncodableCrateDependency;
pub use self::krate_publish::NewCrate as EncodableCrateUpload;
