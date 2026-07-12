use crate::github::next_gh_id;

use chrono::Utc;
use crates_io_database::models::{NewUser, User};
use crates_io_database::schema::oauth_github;
use crates_io_encryption::TokenEncryption;
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

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
    display_name: Option<&'a str>,
    gh_login: &'a str,
}

impl<'a> UserBuilder<'a> {
    /// Create a new instance of the builder. To get a `User`, call `build` (creates an instance in
    /// memory only) or `insert` (creates a record in the database).
    pub fn new() -> Self {
        Self {
            username: "octocat",
            display_name: None,
            gh_login: "octocat",
        }
    }

    pub fn with_username(self, username: &'a str) -> Self {
        Self { username, gh_login: username, ..self }
    }

    pub fn with_display_name(self, display_name: &'a str) -> Self {
        Self {
            display_name: Some(display_name),
            ..self
        }
    }

    pub fn with_gh_username(self, gh_login: &'a str) -> Self {
        Self {
            gh_login: gh_login,
            ..self
        }
    }

    pub fn build(self) -> User {
        User {
            id: 1,
            name: self.display_name.map(ToString::to_string),
            gh_id: 123,
            gh_avatar: None,
            gh_encrypted_token: None,
            account_lock_reason: None,
            account_lock_until: None,
            is_admin: false,
            publish_notifications: true,
            username: self.username.into(),
            gh_login: self.gh_login.into(),
            created_at: None,
        }
    }

    pub fn new_user(self) -> NewUser<'a> {
        NewUser::builder()
            .gh_id(next_gh_id())
            .gh_login(self.gh_login)
            .username(self.username)
            .maybe_name(self.display_name)
            .build()
    }
}

impl Default for UserBuilder<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// A builder to create linked GitHub accounts for a user without having to use `GitHubUser`.
pub struct OauthGithubBuilder<'a> {
    user_id: i32,
    account_id: i64,
    encrypted_token: &'a [u8],
    login: &'a str,
    avatar: Option<&'a str>,
}

impl<'a> OauthGithubBuilder<'a> {
    pub fn for_user(user: &'a User) -> Self {
        Self {
            user_id: user.id,
            account_id: user.gh_id as i64,
            encrypted_token: &ENCRYPTED_TOKEN,
            login: &user.username,
            avatar: None,
        }
    }

    pub fn with_avatar(self, avatar: &'a str) -> Self {
        Self {
            avatar: Some(avatar),
            ..self
        }
    }

    pub fn with_login(self, login: &'a str) -> Self {
        Self { login, ..self }
    }

    pub async fn insert(self, mut conn: &AsyncPgConnection) {
        diesel::insert_into(oauth_github::table)
            .values((
                oauth_github::user_id.eq(self.user_id),
                oauth_github::account_id.eq(self.account_id),
                oauth_github::encrypted_token.eq(self.encrypted_token),
                oauth_github::login.eq(self.login),
                oauth_github::avatar.eq(self.avatar),
                oauth_github::last_sync.eq(Utc::now()),
            ))
            .on_conflict(oauth_github::account_id)
            .do_update()
            .set((
                oauth_github::encrypted_token.eq(excluded(oauth_github::encrypted_token)),
                oauth_github::login.eq(excluded(oauth_github::login)),
                oauth_github::avatar.eq(excluded(oauth_github::avatar)),
                oauth_github::last_sync.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .await
            .unwrap();
    }
}
