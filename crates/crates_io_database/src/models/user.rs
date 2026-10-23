use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::dsl::exists;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::Serialize;

use crate::fns::canon_username;
use crate::models::{Crate, CrateOwner, Email, OwnerKind};
use crate::schema::{crate_owners, emails, oauth_github, users};

// Diesel validates a correlated subquery against the outer query source. `PublicUser` already
// joins `oauth_github`, so reusing that table in this `EXISTS` expression would make it appear
// twice and fail `AppearsInFromClause` validation. This alias gives the subquery a distinct
// query-source identity.
diesel::alias!(
    pub const MATCHING_GITHUB_ACCOUNT: Alias<MatchingGithubAccount> =
        oauth_github as matching_github_account;
);

#[diesel::dsl::auto_type]
pub fn github_username_matches() -> _ {
    let account = MATCHING_GITHUB_ACCOUNT;
    let belongs_to_user = account.field(oauth_github::user_id).eq(users::id);
    let username_matches = account.field(oauth_github::login).eq(users::username);

    exists(account.filter(belongs_to_user).filter(username_matches))
}

/// Finds users whose crates.io username matches the given name.
#[diesel::dsl::auto_type]
pub fn users_by_username<'a>(username: &'a str) -> _ {
    users::table
        .filter(canon_username(users::username).eq(canon_username(username)))
        .order(users::id.desc())
}

/// Public data for a crates.io user.
#[derive(Clone, Debug, HasQuery, Serialize)]
#[diesel(
    table_name = users,
    base_query = users::table.left_join(oauth_github::table),
)]
pub struct PublicUser {
    pub id: i32,
    pub name: Option<String>,
    #[diesel(select_expression = oauth_github::login.nullable())]
    pub gh_login: Option<String>,
    #[diesel(select_expression = oauth_github::avatar.nullable())]
    pub gh_avatar: Option<String>,
    #[diesel(select_expression = github_username_matches())]
    pub github_username_matches: bool,
    pub username: String,
    pub created_at: Option<DateTime<Utc>>,
}

impl PublicUser {
    /// Finds a public user by ID, returning `NotFound` if the user does not exist.
    pub async fn find(mut conn: &AsyncPgConnection, id: i32) -> QueryResult<Self> {
        Self::query()
            .filter(users::id.eq(id))
            .first(&mut conn)
            .await
    }

    pub async fn owning(krate: &Crate, mut conn: &AsyncPgConnection) -> QueryResult<Vec<Self>> {
        CrateOwner::by_owner_kind(OwnerKind::User)
            .inner_join(users::table.left_join(oauth_github::table))
            .select(PublicUser::as_select())
            .filter(crate_owners::crate_id.eq(krate.id))
            .load(&mut conn)
            .await
    }
}

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
    #[diesel(select_expression = oauth_github::login.nullable())]
    pub gh_login: Option<String>,
    #[diesel(select_expression = oauth_github::avatar.nullable())]
    pub gh_avatar: Option<String>,
    #[diesel(select_expression = oauth_github::encrypted_token.nullable())]
    #[serde(skip)]
    pub gh_encrypted_token: Option<Vec<u8>>,
    pub account_lock_reason: Option<String>,
    pub account_lock_until: Option<DateTime<Utc>>,
    pub is_admin: bool,
    pub publish_notifications: bool,
    pub username: String,
    pub created_at: Option<DateTime<Utc>>,
    pub session_epoch: i32,
}

impl User {
    pub async fn find(mut conn: &AsyncPgConnection, id: i32) -> QueryResult<User> {
        User::query()
            .filter(users::id.eq(id))
            .first(&mut conn)
            .await
    }

    pub async fn owning(krate: &Crate, mut conn: &AsyncPgConnection) -> QueryResult<Vec<Self>> {
        CrateOwner::by_owner_kind(OwnerKind::User)
            .inner_join(users::table.left_join(oauth_github::table))
            .select(User::as_select())
            .filter(crate_owners::crate_id.eq(krate.id))
            .load(&mut conn)
            .await
    }

    /// Queries for the email belonging to a particular user.
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
    pub username: &'a str,
    pub name: Option<&'a str>,
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
