use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::models::User;
use crate::schema::oauth_github;

/// Represents an OAuth GitHub account record linked to a user record.
/// Stored in the `oauth_github` table.
#[derive(Associations, Identifiable, Selectable, Queryable, Debug, Clone)]
#[diesel(
    table_name = oauth_github,
    check_for_backend(diesel::pg::Pg),
    primary_key(account_id),
    belongs_to(User),
)]
pub struct OauthGithub {
    /// In the process of being migrated from `users.gh_id`.
    /// GitHub API docs describe this type as int64.
    pub account_id: i64,
    /// In the process of being migrated from `users.gh_avatar`.
    pub avatar: Option<String>,
    /// The OAuth access token from GitHub for this user, encrypted at rest in our database
    pub encrypted_token: Vec<u8>,
    /// The last time we verified with GitHub what the GitHub username for this user was, and
    /// whether the account was valid.
    pub last_sync: DateTime<Utc>,
    /// In the process of being migrated from `users.gh_login`.
    pub login: String,
    /// Foreign key to the `users` table.
    pub user_id: i32,
}

/// Represents a new crates.io user to GitHub user OAuth link to be inserted into the
/// `oauth_github` table.
#[derive(Insertable, Debug, Builder)]
#[diesel(
    table_name = oauth_github,
    check_for_backend(diesel::pg::Pg),
    primary_key(account_id),
    belongs_to(User),
)]
pub struct NewOauthGithub<'a> {
    pub account_id: i64,         // corresponds to users.gh_id
    pub avatar: Option<&'a str>, // corresponds to users.gh_avatar
    pub encrypted_token: &'a [u8],
    #[builder(default = Utc::now())]
    pub last_sync: DateTime<Utc>,
    pub login: &'a str, // corresponds to users.gh_login
    pub user_id: i32,
}

impl NewOauthGithub<'_> {
    pub async fn insert(&self, mut conn: &AsyncPgConnection) -> QueryResult<()> {
        diesel::insert_into(oauth_github::table)
            .values(self)
            .execute(&mut conn)
            .await?;

        Ok(())
    }
}
