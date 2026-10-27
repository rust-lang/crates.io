use crate::github::next_gh_id;

use crates_io_database::models::{NewEmail, NewOauthGithub, NewUser, User};
use crates_io_encryption::TokenEncryption;
use diesel::QueryResult;
use diesel_async::AsyncPgConnection;

use std::sync::LazyLock;

static ENCRYPTED_TOKEN: LazyLock<Vec<u8>> = LazyLock::new(|| {
    TokenEncryption::for_testing()
        .encrypt("some random token")
        .unwrap()
});

/// A builder for test users, which have a linked GitHub account and a verified
/// email address by default.
///
/// `insert()` creates the user in the database, while `build()` returns an
/// in-memory `User` without touching the database.
pub struct UserBuilder<'a> {
    username: &'a str,
    display_name: Option<&'a str>,
    github: Option<GithubAccount<'a>>,
    email: Option<EmailAddress<'a>>,
    is_admin: bool,
}

struct GithubAccount<'a> {
    account_id: Option<i32>,
    login: &'a str,
    avatar: Option<&'a str>,
}

struct EmailAddress<'a> {
    address: Option<&'a str>,
    verified: bool,
}

impl<'a> UserBuilder<'a> {
    /// Creates a builder for a user with the given crates.io username, which
    /// is also the default GitHub login.
    pub fn new(username: &'a str) -> Self {
        Self {
            username,
            display_name: None,
            github: Some(GithubAccount {
                account_id: None,
                login: username,
                avatar: None,
            }),
            email: Some(EmailAddress {
                address: None,
                verified: true,
            }),
            is_admin: false,
        }
    }

    /// Sets the display name, which is unset by default.
    pub fn with_display_name(mut self, display_name: &'a str) -> Self {
        self.display_name = Some(display_name);
        self
    }

    /// Sets the GitHub login independently from the crates.io username.
    #[track_caller]
    pub fn with_gh_login(mut self, login: &'a str) -> Self {
        self.github_mut().login = login;
        self
    }

    /// Sets the GitHub account ID.
    #[track_caller]
    pub fn with_gh_id(mut self, account_id: i32) -> Self {
        self.github_mut().account_id = Some(account_id);
        self
    }

    /// Sets the GitHub avatar URL, which is unset by default.
    #[track_caller]
    pub fn with_gh_avatar(mut self, avatar: &'a str) -> Self {
        self.github_mut().avatar = Some(avatar);
        self
    }

    /// Creates the user without a linked GitHub account.
    pub fn without_github(mut self) -> Self {
        self.github = None;
        self
    }

    #[track_caller]
    fn github_mut(&mut self) -> &mut GithubAccount<'a> {
        let github = self.github.as_mut();
        github.expect("GitHub account options require a linked GitHub account")
    }

    /// Sets the email address, which defaults to `<username>@example.com`.
    #[track_caller]
    pub fn with_email(mut self, address: &'a str) -> Self {
        self.email_mut().address = Some(address);
        self
    }

    /// Sets whether the email address is verified, which it is by default.
    #[track_caller]
    pub fn with_email_verified(mut self, verified: bool) -> Self {
        self.email_mut().verified = verified;
        self
    }

    /// Creates the user without an email address.
    pub fn without_email(mut self) -> Self {
        self.email = None;
        self
    }

    #[track_caller]
    fn email_mut(&mut self) -> &mut EmailAddress<'a> {
        let email = self.email.as_mut();
        email.expect("email options require an email address")
    }

    /// Sets whether the user is an admin or not.
    pub fn with_is_admin(mut self, is_admin: bool) -> Self {
        self.is_admin = is_admin;
        self
    }

    /// Returns an in-memory `User` with ID 1, without touching the database.
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

    /// Inserts the user into the database, links its GitHub account, adds its
    /// email address and returns the user ID.
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

        if let Some(email) = &self.email {
            let address = email
                .address
                .map_or_else(|| format!("{}@example.com", self.username), String::from);

            NewEmail::builder()
                .user_id(id)
                .email(&address)
                .verified(email.verified)
                .build()
                .insert(conn)
                .await?;
        }

        Ok(id)
    }
}
