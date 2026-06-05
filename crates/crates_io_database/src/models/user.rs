use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::Serialize;

use crate::models::{Crate, CrateOwner, Email, Owner, OwnerKind};
use crate::schema::{crate_owners, emails, oauth_github, users};
use crates_io_diesel_helpers::lower;

/// The model representing a row in the `users` database table.
#[derive(Clone, Debug, HasQuery, Identifiable, Serialize)]
#[diesel(
    table_name = users,
    base_query = users::table.left_join(oauth_github::table),
)]
pub struct User {
    pub id: i32,
    pub name: Option<String>,
    #[diesel(select_expression = oauth_github::account_id.nullable())]
    pub gh_id: Option<i64>,
    pub gh_login: String,
    #[diesel(select_expression = oauth_github::avatar.nullable())]
    pub gh_avatar: Option<String>,
    #[diesel(select_expression = oauth_github::encrypted_token.nullable())]
    #[serde(skip)]
    pub gh_encrypted_token: Option<Vec<u8>>,
    pub account_lock_reason: Option<String>,
    pub account_lock_until: Option<DateTime<Utc>>,
    pub is_admin: bool,
    pub publish_notifications: bool,
}

impl User {
    pub async fn find(mut conn: &AsyncPgConnection, id: i32) -> QueryResult<User> {
        User::query()
            .filter(users::id.eq(id))
            .first(&mut conn)
            .await
    }

    pub async fn find_by_login(mut conn: &AsyncPgConnection, login: &str) -> QueryResult<User> {
        User::query()
            .filter(lower(users::gh_login).eq(login.to_lowercase()))
            // This ordering will be unnecessary when we switch to crates.io usernames that
            // are unique.
            .order(users::id.desc())
            .first(&mut conn)
            .await
    }

    pub async fn owning(krate: &Crate, mut conn: &AsyncPgConnection) -> QueryResult<Vec<Owner>> {
        let users = CrateOwner::by_owner_kind(OwnerKind::User)
            .inner_join(users::table.left_join(oauth_github::table))
            .select(User::as_select())
            .filter(crate_owners::crate_id.eq(krate.id))
            .load(&mut conn)
            .await?
            .into_iter()
            .map(Owner::User);

        Ok(users.collect())
    }

    /// Queries the database for the verified emails
    /// belonging to a given user
    pub async fn verified_email(
        &self,
        mut conn: &AsyncPgConnection,
    ) -> QueryResult<Option<String>> {
        Email::belonging_to(self)
            .select(emails::email)
            .filter(emails::verified.eq(true))
            .first(&mut conn)
            .await
            .optional()
    }

    /// Queries for the email belonging to a particular user
    pub async fn email(&self, mut conn: &AsyncPgConnection) -> QueryResult<Option<String>> {
        Email::belonging_to(self)
            .select(emails::email)
            .first(&mut conn)
            .await
            .optional()
    }
}

/// Represents a new user record insertable to the `users` table
#[derive(Insertable, Debug, Builder)]
#[diesel(table_name = users, check_for_backend(diesel::pg::Pg))]
pub struct NewUser<'a> {
    // Needs to be set until we decide to drop the database constraint, but should not be read.
    pub gh_id: i32,
    pub gh_login: &'a str,
    pub name: Option<&'a str>,
    // Needs to be set until we decide to drop the database constraint, but should not be read.
    pub gh_encrypted_token: &'a [u8],
}

impl NewUser<'_> {
    /// Inserts the user into the database, or fails if the user already exists.
    pub async fn insert(&self, mut conn: &AsyncPgConnection) -> QueryResult<i32> {
        diesel::insert_into(users::table)
            .values(self)
            .returning(users::id)
            .get_result(&mut conn)
            .await
    }
}

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
    /// GitHub API docs describe this type as int64
    pub account_id: i64,
    /// In the process of being migrated from `users.gh_avatar`.
    pub avatar: Option<String>,
    /// In the process of being migrated from `users.gh_encrypted_token`.
    pub encrypted_token: Vec<u8>,
    /// The last time we verified with GitHub what the GitHub username for this user was, and
    /// whether the account was valid
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
pub struct OauthGithubUpdate<'a> {
    pub account_id: i64,           // corresponds to users.gh_id
    pub avatar: Option<&'a str>,   // corresponds to users.gh_avatar
    pub encrypted_token: &'a [u8], // corresponds to users.gh_encrypted_token
    pub login: &'a str,            // corresponds to users.gh_login
}

impl OauthGithubUpdate<'_> {
    /// Updates an existing record of the associated GitHub account info into the database.
    /// Does not insert, because to insert, we must first have a `users` record to associate to.
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
    pub async fn update(&self, mut conn: &AsyncPgConnection) -> QueryResult<OauthGithub> {
        diesel::update(oauth_github::table)
            .filter(oauth_github::account_id.eq(self.account_id))
            .set((
                oauth_github::encrypted_token.eq(self.encrypted_token),
                oauth_github::login.eq(self.login),
                oauth_github::avatar.eq(self.avatar),
                oauth_github::last_sync.eq(Utc::now()),
            ))
            .get_result(&mut conn)
            .await
    }

    pub async fn insert(
        &self,
        mut conn: &AsyncPgConnection,
        user_id: i32,
    ) -> QueryResult<OauthGithub> {
        diesel::insert_into(oauth_github::table)
            .values((
                oauth_github::user_id.eq(user_id),
                oauth_github::account_id.eq(self.account_id),
                oauth_github::encrypted_token.eq(self.encrypted_token),
                oauth_github::login.eq(self.login),
                oauth_github::avatar.eq(self.avatar),
                oauth_github::last_sync.eq(Utc::now()),
            ))
            .get_result(&mut conn)
            .await
    }
}
