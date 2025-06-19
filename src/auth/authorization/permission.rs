use crates_io_database::models::{Crate, Owner};

pub enum Permission<'a> {
    ListApiTokens,
    CreateApiToken,
    ReadApiToken,
    RevokeApiToken,
    RevokeCurrentApiToken,

    PublishNew {
        name: &'a str,
    },
    PublishUpdate {
        krate: &'a Crate,
    },
    DeleteCrate {
        krate: &'a Crate,
        owners: &'a [Owner],
    },

    ModifyOwners {
        krate: &'a Crate,
        owners: &'a [Owner],
    },

    ListCrateOwnerInvitations,
    ListOwnCrateOwnerInvitations,
    HandleCrateOwnerInvitation,

    ListFollowedCrates,
    ReadFollowState,
    FollowCrate,
    UnfollowCrate,

    ListTrustPubGitHubConfigs {
        krate: &'a Crate,
    },
    CreateTrustPubGitHubConfig {
        user_owner_ids: Vec<i32>,
    },
    DeleteTrustPubGitHubConfig {
        user_owner_ids: Vec<i32>,
    },

    ReadUser,
    UpdateUser,

    UpdateVersion {
        krate: &'a Crate,
    },
    YankVersion {
        krate: &'a Crate,
    },
    UnyankVersion {
        krate: &'a Crate,
    },

    ResendEmailVerification,
    UpdateEmailNotifications,
    ListUpdates,

    RebuildDocs {
        krate: &'a Crate,
    },
}

impl Permission<'_> {
    #[allow(clippy::match_like_matches_macro)]
    pub(in crate::auth) fn allowed_for_admin(&self) -> bool {
        match self {
            Permission::YankVersion { .. } => true,
            Permission::UnyankVersion { .. } => true,
            _ => false,
        }
    }
}
