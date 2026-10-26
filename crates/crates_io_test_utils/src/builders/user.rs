use crate::github::next_gh_id;

use chrono::Utc;
use crates_io_database::models::{NewOauthGithub, NewUser, User};
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
    github: Option<GithubAccount<'a>>,
    is_admin: bool,
}

struct GithubAccount<'a> {
    account_id: i32,
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
                account_id: next_gh_id(),
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
        self.github_mut().account_id = account_id;
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
            gh_id: github.map(|github| github.account_id.into()),
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

    pub fn new_user(&self) -> NewUser<'a> {
        let gh_id = match &self.github {
            Some(github) => github.account_id,
            // Production stores `-1` for users whose GitHub account was deleted.
            None => -1,
        };

        NewUser::builder()
            .gh_id(gh_id)
            .username(self.username)
            .maybe_name(self.display_name)
            .is_admin(self.is_admin)
            .build()
    }

    /// Inserts the user into the database, links its GitHub account and
    /// returns the user ID.
    pub async fn insert(self, conn: &AsyncPgConnection) -> QueryResult<i32> {
        let id = self.new_user().insert(conn).await?;

        if let Some(github) = &self.github {
            NewOauthGithub::builder()
                .account_id(github.account_id.into())
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
            account_id: next_gh_id() as i64,
            encrypted_token: &ENCRYPTED_TOKEN,
            login: user.gh_login.as_ref().unwrap_or(&user.username),
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

    pub fn with_account_id(self, account_id: i64) -> Self {
        Self { account_id, ..self }
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
