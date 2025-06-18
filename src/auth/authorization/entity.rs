use crate::auth::authorization::trustpub::AuthorizedTrustPub;
use crate::auth::authorization::user::AuthorizedUser;
use crates_io_database::models::{ApiToken, User};

pub enum AuthorizedEntity {
    User(Box<AuthorizedUser<Option<ApiToken>>>),
    TrustPub(AuthorizedTrustPub),
}

impl AuthorizedEntity {
    pub fn user_auth(&self) -> Option<&AuthorizedUser<Option<ApiToken>>> {
        match self {
            AuthorizedEntity::User(auth) => Some(auth),
            AuthorizedEntity::TrustPub(_) => None,
        }
    }

    pub fn user(&self) -> Option<&User> {
        self.user_auth().map(|auth| auth.user())
    }

    pub fn user_id(&self) -> Option<i32> {
        self.user_auth().map(|auth| auth.user_id())
    }
}
