use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::fns::lower;
use crate::models::User;
use crate::schema::oauth_github;

/// The model representing a row in the `oauth_github` database table, linked to a user record.
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
    /// In the process of being migrated from `users.gh_encrypted_token`.
    pub encrypted_token: Vec<u8>,
    /// The last time we verified with GitHub what the GitHub username for this user was, and
    /// whether the account was valid.
    pub last_sync: DateTime<Utc>,
    /// In the process of being migrated from `users.gh_login`.
    pub login: String,
    /// Foreign key to the `users` table.
    pub user_id: i32,
}

impl OauthGithub {
    pub async fn find_by_login(
        mut conn: &AsyncPgConnection,
        login: &str,
    ) -> QueryResult<OauthGithub> {
        oauth_github::table
            .filter(lower(oauth_github::login).eq(login.to_lowercase()))
            .select(OauthGithub::as_select())
            .first(&mut conn)
            .await
    }
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
    pub account_id: i64,           // corresponds to users.gh_id
    pub avatar: Option<&'a str>,   // corresponds to users.gh_avatar
    pub encrypted_token: &'a [u8], // corresponds to users.gh_encrypted_token
    pub login: &'a str,            // corresponds to users.gh_login
    pub user_id: i32,
}

impl NewOauthGithub<'_> {
    /// Inserts the associated GitHub account info into the database, or updates an existing record.
    ///
    /// GitHub `account_id` is the primary key of the `oauth_github` table, and comes from GitHub.
    ///
    /// Each GitHub account ID can only be associated with one crates.io account, so that we know
    /// who to log in when we get a GitHub oAuth response.
    ///
    /// If this function gets an `account_id` conflict, it does not and should not update the
    /// `user_id` to that of the currently-logged-in crates.io user's ID because that would mean
    /// that GitHub account has already been associated with a different crates.io account. In that
    /// case, the currently-logged-in crates.io user should be logged out and the crates.io user
    /// already associated with this GitHub user should be logged in.
    ///
    /// We may eventually implement the ability to associate multiple GitHub accounts with one
    /// crates.io account.
    ///
    /// This function should be called if there is no current user and should update the encrypted
    /// token, login, or avatar if those have changed.
    pub async fn insert_or_update(&self, mut conn: &AsyncPgConnection) -> QueryResult<OauthGithub> {
        diesel::insert_into(oauth_github::table)
            .values(self)
            .on_conflict(oauth_github::account_id)
            .do_update()
            .set((
                oauth_github::encrypted_token.eq(excluded(oauth_github::encrypted_token)),
                oauth_github::login.eq(excluded(oauth_github::login)),
                oauth_github::avatar.eq(excluded(oauth_github::avatar)),
                oauth_github::last_sync.eq(Utc::now()),
            ))
            .get_result(&mut conn)
            .await
    }
}
