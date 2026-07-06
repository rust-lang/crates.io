use crate::github::next_gh_id;

use crates_io_database::models::{NewUser, User};
use crates_io_encryption::TokenEncryption;

use std::sync::LazyLock;

static ENCRYPTED_TOKEN: LazyLock<Vec<u8>> = LazyLock::new(|| {
    TokenEncryption::for_testing()
        .encrypt("some random token")
        .unwrap()
});

/// A builder to create user instances for the purpose of not using the database or inserting
/// directly into the database.
///
/// If you want to test logic that happens as part of signing up or logging in,
pub struct UserBuilder<'a> {
    username: &'a str,
}

impl<'a> UserBuilder<'a> {
    /// Create a new instance of the builder. To get a `User`, call `build` (creates an instance in
    /// memory only) or `insert` (creates a record in the database).
    pub fn new() -> Self {
        Self {
            username: "octocat",
        }
    }

    pub fn with_username(self, username: &'a str) -> Self {
        Self { username }
    }

    pub fn build(self) -> User {
        User {
            id: 1,
            gh_login: self.username.into(),
            name: Some("The Octocat".into()),
            gh_id: 123,
            gh_avatar: None,
            gh_encrypted_token: vec![],
            account_lock_reason: None,
            account_lock_until: None,
            is_admin: false,
            publish_notifications: true,
            username: self.username.into(),
            created_at: None,
        }
    }

    pub fn new_user(self) -> NewUser<'a> {
        NewUser::builder()
            .gh_id(next_gh_id())
            .gh_login(self.username)
            .username(self.username)
            .gh_encrypted_token(&ENCRYPTED_TOKEN)
            .build()
    }
}

impl Default for UserBuilder<'_> {
    fn default() -> Self {
        Self::new()
    }
}
