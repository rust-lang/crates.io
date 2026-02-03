use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::Integer;
use diesel::upsert::excluded;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::Serialize;

use crate::models::{Crate, CrateOwner, Email, Owner, OwnerKind};
use crate::schema::{crate_owners, emails, oauth_github, users};
use crates_io_diesel_helpers::lower;

/// The model representing a row in the `users` database table.
#[derive(Clone, Debug, HasQuery, Identifiable, Serialize)]
pub struct User {
    pub id: i32,
    pub name: Option<String>,
    pub gh_id: i32,
    pub gh_login: String,
    pub gh_avatar: Option<String>,
    #[serde(skip)]
    pub gh_encrypted_token: Vec<u8>,
    pub account_lock_reason: Option<String>,
    pub account_lock_until: Option<DateTime<Utc>>,
    pub is_admin: bool,
    pub publish_notifications: bool,
}

impl User {
    pub async fn find(conn: &mut AsyncPgConnection, id: i32) -> QueryResult<User> {
        User::query().find(id).first(conn).await
    }

    pub async fn find_all_by_login(
        conn: &mut AsyncPgConnection,
        login: &str,
    ) -> QueryResult<Vec<User>> {
        User::query()
            .inner_join(oauth_github::table)
            .filter(lower(oauth_github::login).eq(lower(login)))
            .order(oauth_github::account_id.desc())
            .load(conn)
            .await
    }

    pub async fn owning(krate: &Crate, conn: &mut AsyncPgConnection) -> QueryResult<Vec<Owner>> {
        let users = CrateOwner::by_owner_kind(OwnerKind::User)
            .inner_join(users::table)
            .select(User::as_select())
            .filter(crate_owners::crate_id.eq(krate.id))
            .load(conn)
            .await?
            .into_iter()
            .map(Owner::User);

        Ok(users.collect())
    }

    /// Queries the database for the verified emails
    /// belonging to a given user
    pub async fn verified_email(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> QueryResult<Option<String>> {
        Email::belonging_to(self)
            .select(emails::email)
            .filter(emails::verified.eq(true))
            .first(conn)
            .await
            .optional()
    }

    /// Queries for the email belonging to a particular user
    pub async fn email(&self, conn: &mut AsyncPgConnection) -> QueryResult<Option<String>> {
        Email::belonging_to(self)
            .select(emails::email)
            .first(conn)
            .await
            .optional()
    }
}

/// Represents a new user record insertable to the `users` table
#[derive(Insertable, Debug, Builder)]
#[diesel(table_name = users, check_for_backend(diesel::pg::Pg))]
pub struct NewUser<'a> {
    pub gh_id: i32,
    pub gh_login: &'a str,
    pub name: Option<&'a str>,
    pub gh_avatar: Option<&'a str>,
    pub gh_encrypted_token: &'a [u8],
}

impl NewUser<'_> {
    /// Inserts the user into the database, or fails if the user already exists.
    pub async fn insert(&self, conn: &mut AsyncPgConnection) -> QueryResult<User> {
        diesel::insert_into(users::table)
            .values(self)
            .returning(User::as_returning())
            .get_result(conn)
            .await
    }

    /// Inserts the user into the database, or updates an existing one.
    pub async fn insert_or_update(&self, conn: &mut AsyncPgConnection) -> QueryResult<User> {
        diesel::insert_into(users::table)
            .values(self)
            // We need the `WHERE gh_id > 0` condition here because `gh_id` set
            // to `-1` indicates that we were unable to find a GitHub ID for
            // the associated GitHub login at the time that we backfilled
            // GitHub IDs. Therefore, there are multiple records in production
            // that have a `gh_id` of `-1` so we need to exclude those when
            // considering uniqueness of `gh_id` values. The `> 0` condition isn't
            // necessary for most fields in the database to be used as a conflict
            // target :)
            .on_conflict(sql::<Integer>("(gh_id) WHERE gh_id > 0"))
            .do_update()
            .set((
                users::gh_login.eq(excluded(users::gh_login)),
                users::name.eq(excluded(users::name)),
                users::gh_avatar.eq(excluded(users::gh_avatar)),
                users::gh_encrypted_token.eq(excluded(users::gh_encrypted_token)),
            ))
            .returning(User::as_returning())
            .get_result(conn)
            .await
    }
}

/// A linked account of any type supported for login and associated with one `User`.
pub enum LinkedAccount {
    Github(OauthGithub),
}

/// A user with their linked accounts, retrieved with more database queries. If you don't need
/// linked accounts, use `User`.
pub struct UserWithLinkedAccounts {
    pub user: User,
    pub linked_accounts: Vec<LinkedAccount>,
}

impl UserWithLinkedAccounts {
    /// For all users specified, look up all linked accounts associated with the users and return
    /// them collated with their associated user.
    pub async fn find_all_by_users(
        conn: &mut AsyncPgConnection,
        users: Vec<User>,
    ) -> QueryResult<Vec<UserWithLinkedAccounts>> {
        let oauth_github_accounts = OauthGithub::belonging_to(&users)
            .select(OauthGithub::as_select())
            .load(conn)
            .await?;

        Ok(oauth_github_accounts
            .grouped_by(&users)
            .into_iter()
            .zip(users)
            .map(|(github_accounts, user)| Self {
                user,
                linked_accounts: github_accounts
                    .into_iter()
                    .map(LinkedAccount::Github)
                    .collect(),
            })
            .collect())
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
    pub async fn insert_or_update(&self, conn: &mut AsyncPgConnection) -> QueryResult<OauthGithub> {
        diesel::insert_into(oauth_github::table)
            .values(self)
            .on_conflict(oauth_github::account_id)
            .do_update()
            .set((
                oauth_github::encrypted_token.eq(excluded(oauth_github::encrypted_token)),
                oauth_github::login.eq(excluded(oauth_github::login)),
                oauth_github::avatar.eq(excluded(oauth_github::avatar)),
            ))
            .get_result(conn)
            .await
    }
}
