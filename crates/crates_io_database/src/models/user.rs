use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::Integer;
use diesel::upsert::excluded;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use secrecy::SecretString;

use crate::models::{Crate, CrateOwner, Email, Owner, OwnerKind};
use crate::schema::{crate_owners, emails, linked_accounts, users};
use crates_io_diesel_helpers::{lower, pg_enum};

/// The model representing a row in the `users` database table.
#[derive(Clone, Debug, Queryable, Identifiable, Selectable)]
pub struct User {
    pub id: i32,
    #[diesel(deserialize_as = String)]
    pub gh_access_token: SecretString,
    pub gh_login: String,
    pub name: Option<String>,
    pub gh_avatar: Option<String>,
    pub gh_id: i32,
    pub account_lock_reason: Option<String>,
    pub account_lock_until: Option<DateTime<Utc>>,
    pub is_admin: bool,
    pub publish_notifications: bool,
    pub username: Option<String>,
}

impl User {
    pub async fn find(conn: &mut AsyncPgConnection, id: i32) -> QueryResult<User> {
        users::table.find(id).first(conn).await
    }

    pub async fn find_by_login(conn: &mut AsyncPgConnection, login: &str) -> QueryResult<User> {
        users::table
            .filter(lower(users::gh_login).eq(login.to_lowercase()))
            .filter(users::gh_id.ne(-1))
            .order(users::gh_id.desc())
            .first(conn)
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
    pub username: Option<&'a str>,
    pub gh_avatar: Option<&'a str>,
    pub gh_access_token: &'a str,
}

impl NewUser<'_> {
    /// Inserts the user into the database, or fails if the user already exists.
    pub async fn insert(&self, conn: &mut AsyncPgConnection) -> QueryResult<User> {
        diesel::insert_into(users::table)
            .values(self)
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
                users::username.eq(excluded(users::username)),
                users::name.eq(excluded(users::name)),
                users::gh_avatar.eq(excluded(users::gh_avatar)),
                users::gh_access_token.eq(excluded(users::gh_access_token)),
            ))
            .get_result(conn)
            .await
    }
}

// Supported OAuth providers. Currently only GitHub.
pg_enum! {
    pub enum AccountProvider {
        Github = 0,
    }
}

/// Represents an OAuth account record linked to a user record.
#[derive(Associations, Identifiable, Selectable, Queryable, Debug, Clone)]
#[diesel(
    table_name = linked_accounts,
    check_for_backend(diesel::pg::Pg),
    primary_key(provider, account_id),
    belongs_to(User),
)]
pub struct LinkedAccount {
    pub user_id: i32,
    pub provider: AccountProvider,
    pub account_id: i32, // corresponds to user.gh_id
    #[diesel(deserialize_as = String)]
    pub access_token: SecretString, // corresponds to user.gh_access_token
    pub login: String,   // corresponds to user.gh_login
    pub avatar: Option<String>, // corresponds to user.gh_avatar
}

/// Represents a new linked account record insertable to the `linked_accounts` table
#[derive(Insertable, Debug, Builder)]
#[diesel(
    table_name = linked_accounts,
    check_for_backend(diesel::pg::Pg),
    primary_key(provider, account_id),
    belongs_to(User),
)]
pub struct NewLinkedAccount<'a> {
    pub user_id: i32,
    pub provider: AccountProvider,
    pub account_id: i32,         // corresponds to user.gh_id
    pub access_token: &'a str,   // corresponds to user.gh_access_token
    pub login: &'a str,          // corresponds to user.gh_login
    pub avatar: Option<&'a str>, // corresponds to user.gh_avatar
}

impl NewLinkedAccount<'_> {
    /// Inserts the linked account into the database, or updates an existing one.
    ///
    /// This is to be used for logging in when there is no currently logged-in user, as opposed to
    /// adding another linked account to a currently-logged-in user. The logic for adding another
    /// linked account (when that ability gets added) will need to ensure that a particular
    /// (provider, account_id) combo (ex: GitHub account with GitHub ID 1234) is only associated
    /// with one crates.io account, so that we know what crates.io account to log in when we get an
    /// oAuth request from GitHub ID 1234. In other words, we should NOT be updating the user_id on
    /// an existing (provider, account_id) row when starting from a currently-logged-in crates.io \
    /// user because that would mean that oAuth account has already been associated with a
    /// different crates.io account.
    ///
    /// This function should be called if there is no current user and should update, say, the
    /// access token, login, or avatar if those have changed.
    pub async fn insert_or_update(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> QueryResult<LinkedAccount> {
        diesel::insert_into(linked_accounts::table)
            .values(self)
            .on_conflict((linked_accounts::provider, linked_accounts::account_id))
            .do_update()
            .set((
                linked_accounts::access_token.eq(excluded(linked_accounts::access_token)),
                linked_accounts::login.eq(excluded(linked_accounts::login)),
                linked_accounts::avatar.eq(excluded(linked_accounts::avatar)),
            ))
            .get_result(conn)
            .await
    }
}
