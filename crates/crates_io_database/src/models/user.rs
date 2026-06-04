use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::Integer;
use diesel::upsert::excluded;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::Serialize;

use crate::fns::lower;
use crate::models::{Crate, CrateOwner, Email, Owner, OwnerKind};
use crate::schema::{crate_owners, emails, oauth_github, users};

/// The model representing a row in the `users` database table.
#[derive(Clone, Debug, HasQuery, Identifiable, Serialize)]
#[diesel(
    table_name = users,
    base_query = users::table.left_join(oauth_github::table),
)]
pub struct User {
    pub id: i32,
    pub name: Option<String>,
    pub gh_id: i32,
    pub gh_login: String,
    #[diesel(select_expression = oauth_github::avatar.nullable())]
    pub gh_avatar: Option<String>,
    #[serde(skip)]
    pub gh_encrypted_token: Vec<u8>,
    pub account_lock_reason: Option<String>,
    pub account_lock_until: Option<DateTime<Utc>>,
    pub is_admin: bool,
    pub publish_notifications: bool,
    pub username: String,
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
            .filter(users::gh_id.ne(-1))
            .order(users::gh_id.desc())
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
    pub gh_id: i32,
    pub gh_login: &'a str,
    pub username: &'a str,
    pub name: Option<&'a str>,
    pub gh_avatar: Option<&'a str>,
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

    /// Inserts the user into the database, or updates an existing one.
    pub async fn insert_or_update(&self, mut conn: &AsyncPgConnection) -> QueryResult<i32> {
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
                users::gh_encrypted_token.eq(excluded(users::gh_encrypted_token)),
            ))
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
