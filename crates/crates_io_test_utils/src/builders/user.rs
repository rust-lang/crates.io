use crate::github::next_gh_id;

use crates_io_database::models::{NewOauthGithub, NewUser, User};
use crates_io_encryption::TokenEncryption;
use diesel::QueryResult;
use diesel_async::AsyncPgConnection;

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
    github: Option<GithubAccount<'a>>,
    is_admin: bool,
}

struct GithubAccount<'a> {
    account_id: Option<i32>,
    login: &'a str,
    avatar: Option<&'a str>,
}

impl<'a> UserBuilder<'a> {
    /// Create a new instance of the builder. To get a `User`, call `build` (creates an instance in
    /// memory only) or `insert` (creates a record in the database).
    pub fn new(username: &'a str) -> Self {
        Self {
            username,
            display_name: None,
            github: Some(GithubAccount {
                account_id: None,
                login: username,
                avatar: None,
            }),
            is_admin: false,
        }
    }

    pub fn with_display_name(self, display_name: &'a str) -> Self {
        Self {
            display_name: Some(display_name),
            ..self
        }
    }

    /// Sets the GitHub login independently from the crates.io username.
    pub fn with_gh_login(mut self, login: &'a str) -> Self {
        self.github_mut().login = login;
        self
    }

    /// Sets the GitHub account ID.
    pub fn with_gh_id(mut self, account_id: i32) -> Self {
        self.github_mut().account_id = Some(account_id);
        self
    }

    pub fn with_gh_avatar(mut self, avatar: &'a str) -> Self {
        self.github_mut().avatar = Some(avatar);
        self
    }

    pub fn without_github(mut self) -> Self {
        self.github = None;
        self
    }

    fn github_mut(&mut self) -> &mut GithubAccount<'a> {
        let github = self.github.as_mut();
        github.expect("GitHub account options require a linked GitHub account")
    }

    /// Sets whether the user is an admin or not.
    pub fn with_is_admin(self, is_admin: bool) -> Self {
        Self { is_admin, ..self }
    }

    pub fn build(self) -> User {
        let github = self.github.as_ref();

        User {
            id: 1,
            gh_login: github.map(|github| github.login.into()),
            name: self.display_name.map(ToString::to_string),
            gh_id: github.map(|github| github.account_id.unwrap_or_else(next_gh_id).into()),
            gh_avatar: github.and_then(|github| github.avatar.map(ToString::to_string)),
            gh_encrypted_token: None,
            account_lock_reason: None,
            account_lock_until: None,
            is_admin: self.is_admin,
            publish_notifications: true,
            username: self.username.into(),
            created_at: None,
            session_epoch: 0,
        }
    }

    /// Inserts the user into the database, links its GitHub account and
    /// returns the user ID.
    pub async fn insert(self, conn: &AsyncPgConnection) -> QueryResult<i32> {
        let gh_id = match &self.github {
            Some(github) => github.account_id.unwrap_or_else(next_gh_id),
            // Production stores `-1` for users whose GitHub account was deleted.
            None => -1,
        };

        let id = NewUser::builder()
            .gh_id(gh_id)
            .username(self.username)
            .maybe_name(self.display_name)
            .is_admin(self.is_admin)
            .build()
            .insert(conn)
            .await?;

        if let Some(github) = &self.github {
            NewOauthGithub::builder()
                .account_id(gh_id.into())
                .login(github.login)
                .maybe_avatar(github.avatar)
                .encrypted_token(&ENCRYPTED_TOKEN)
                .user_id(id)
                .build()
                .insert(conn)
                .await?;
        }

        Ok(id)
    }
}
