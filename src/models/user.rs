use chrono::NaiveDateTime;
use diesel::prelude::*;
use std::borrow::Cow;

use crate::app::App;
use crate::email::Emails;
use crate::util::errors::AppResult;

use crate::models::{ApiToken, Crate, CrateOwner, Email, NewEmail, Owner, OwnerKind, Rights};
use crate::schema::{crate_owners, emails, users};

/// The model representing a row in the `users` database table.
#[derive(Clone, Debug, PartialEq, Eq, Queryable, Identifiable, AsChangeset, Associations)]
pub struct User {
    pub id: i32,
    pub gh_access_token: String,
    pub gh_login: String,
    pub name: Option<String>,
    pub gh_avatar: Option<String>,
    pub gh_id: i32,
    pub account_lock_reason: Option<String>,
    pub account_lock_until: Option<NaiveDateTime>,
}

/// Represents a new user record insertable to the `users` table
#[derive(Insertable, Debug, Default)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub gh_id: i32,
    pub gh_login: &'a str,
    pub name: Option<&'a str>,
    pub gh_avatar: Option<&'a str>,
    pub gh_access_token: Cow<'a, str>,
}

impl<'a> NewUser<'a> {
    pub fn new(
        gh_id: i32,
        gh_login: &'a str,
        name: Option<&'a str>,
        gh_avatar: Option<&'a str>,
        gh_access_token: &'a str,
    ) -> Self {
        NewUser {
            gh_id,
            gh_login,
            name,
            gh_avatar,
            gh_access_token: Cow::Borrowed(gh_access_token),
        }
    }

    /// Inserts the user into the database, or updates an existing one.
    pub fn create_or_update(
        &self,
        email: Option<&'a str>,
        emails: &Emails,
        conn: &PgConnection,
    ) -> QueryResult<User> {
        use crate::schema::users::dsl::*;
        use diesel::dsl::sql;
        use diesel::insert_into;
        use diesel::pg::upsert::excluded;
        use diesel::sql_types::Integer;

        conn.transaction(|| {
            let user: User = insert_into(users)
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
                    gh_login.eq(excluded(gh_login)),
                    name.eq(excluded(name)),
                    gh_avatar.eq(excluded(gh_avatar)),
                    gh_access_token.eq(excluded(gh_access_token)),
                ))
                .get_result(conn)?;

            // To send the user an account verification email
            if let Some(user_email) = email {
                let new_email = NewEmail {
                    user_id: user.id,
                    email: user_email,
                };

                let token: Option<String> = insert_into(emails::table)
                    .values(&new_email)
                    .on_conflict_do_nothing()
                    .returning(emails::token)
                    .get_result(conn)
                    .optional()?;

                if let Some(token) = token {
                    // Swallows any error. Some users might insert an invalid email address here.
                    let _ = emails.send_user_confirm(user_email, &user.gh_login, &token);
                }
            }

            Ok(user)
        })
    }
}

impl User {
    pub fn find(conn: &PgConnection, id: i32) -> QueryResult<User> {
        users::table.find(id).first(conn)
    }

    /// Queries the database for a user with a certain `api_token` value.
    pub fn find_by_api_token(conn: &PgConnection, token: &str) -> AppResult<User> {
        let api_token = ApiToken::find_by_api_token(conn, token)?;

        Ok(Self::find(conn, api_token.user_id)?)
    }

    pub fn owning(krate: &Crate, conn: &PgConnection) -> QueryResult<Vec<Owner>> {
        let users = CrateOwner::by_owner_kind(OwnerKind::User)
            .inner_join(users::table)
            .select(users::all_columns)
            .filter(crate_owners::crate_id.eq(krate.id))
            .load(conn)?
            .into_iter()
            .map(Owner::User);

        Ok(users.collect())
    }

    /// Given this set of owners, determines the strongest rights the
    /// user has.
    ///
    /// Shortcircuits on `Full` because you can't beat it. In practice we'll always
    /// see `[user, user, user, ..., team, team, team]`, so we could shortcircuit on
    /// `Publish` as well, but this is a non-obvious invariant so we don't bother.
    /// Sweet free optimization if teams are proving burdensome to check.
    /// More than one team isn't really expected, though.
    pub fn rights(&self, app: &App, owners: &[Owner]) -> AppResult<Rights> {
        let mut best = Rights::None;
        for owner in owners {
            match *owner {
                Owner::User(ref other_user) => {
                    if other_user.id == self.id {
                        return Ok(Rights::Full);
                    }
                }
                Owner::Team(ref team) => {
                    if team.contains_user(app, self)? {
                        best = Rights::Publish;
                    }
                }
            }
        }
        Ok(best)
    }

    /// Queries the database for the verified emails
    /// belonging to a given user
    pub fn verified_email(&self, conn: &PgConnection) -> QueryResult<Option<String>> {
        Email::belonging_to(self)
            .select(emails::email)
            .filter(emails::verified.eq(true))
            .first(&*conn)
            .optional()
    }

    /// Queries for the email belonging to a particular user
    pub fn email(&self, conn: &PgConnection) -> AppResult<Option<String>> {
        Ok(Email::belonging_to(self)
            .select(emails::email)
            .first(&*conn)
            .optional()?)
    }
}
