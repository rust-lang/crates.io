use crates_io_database::models::User;

/// A builder to create user instances for the purpose of not using the database or inserting
/// directly into the database.
///
/// If you want to test logic that happens as part of signing up or logging in,
pub struct UserBuilder {}

impl UserBuilder {
    /// Create a new instance of the builder. To get a `User`, call `build` (creates an instance in
    /// memory only) or `insert` (creates a record in the database).
    pub fn new() -> Self {
        Self {}
    }

    pub fn build(self) -> User {
        User {
            id: 1,
            gh_login: "octocat".into(),
            name: Some("The Octocat".into()),
            gh_id: 123,
            gh_avatar: None,
            gh_encrypted_token: vec![],
            account_lock_reason: None,
            account_lock_until: None,
            is_admin: false,
            publish_notifications: true,
            username: "octocat".into(),
            created_at: None,
        }
    }
}

impl Default for UserBuilder {
    fn default() -> Self {
        Self::new()
    }
}
