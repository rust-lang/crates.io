use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::Integer;
use diesel::upsert::excluded;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use secrecy::SecretString;

use crate::models::{Crate, CrateOwner, Email, Owner, OwnerKind};
use crate::schema::{crate_owners, emails, users};
use crates_io_diesel_helpers::lower;

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
                users::name.eq(excluded(users::name)),
                users::gh_avatar.eq(excluded(users::gh_avatar)),
                users::gh_access_token.eq(excluded(users::gh_access_token)),
            ))
            .get_result(conn)
            .await
    }
}
