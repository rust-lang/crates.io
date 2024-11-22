use chrono::NaiveDateTime;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection};
use secrecy::SecretString;

use crate::app::App;
use crate::controllers::user::update::UserConfirmEmail;
use crate::email::Emails;
use crate::util::errors::AppResult;

use crate::models::{Crate, CrateOwner, Email, NewEmail, Owner, OwnerKind, Rights};
use crate::schema::{crate_owners, emails, users};
use crate::sql::lower;
use crate::util::diesel::prelude::*;
use crate::util::diesel::Conn;

/// The model representing a row in the `users` database table.
#[derive(Clone, Debug, PartialEq, Eq, Queryable, Identifiable, AsChangeset, Selectable)]
pub struct User {
    pub id: i32,
    pub gh_access_token: String,
    pub gh_login: String,
    pub name: Option<String>,
    pub gh_avatar: Option<String>,
    pub gh_id: i32,
    pub account_lock_reason: Option<String>,
    pub account_lock_until: Option<NaiveDateTime>,
    pub is_admin: bool,
    pub publish_notifications: bool,
}

impl User {
    pub async fn find(conn: &mut AsyncPgConnection, id: i32) -> QueryResult<User> {
        use diesel_async::RunQueryDsl;

        users::table.find(id).first(conn).await
    }

    pub fn find_by_login(conn: &mut impl Conn, login: &str) -> QueryResult<User> {
        use diesel::RunQueryDsl;

        users::table
            .filter(lower(users::gh_login).eq(login.to_lowercase()))
            .filter(users::gh_id.ne(-1))
            .order(users::gh_id.desc())
            .first(conn)
    }

    pub async fn async_find_by_login(
        conn: &mut AsyncPgConnection,
        login: &str,
    ) -> QueryResult<User> {
        use diesel_async::RunQueryDsl;

        users::table
            .filter(lower(users::gh_login).eq(login.to_lowercase()))
            .filter(users::gh_id.ne(-1))
            .order(users::gh_id.desc())
            .first(conn)
            .await
    }

    pub async fn owning(krate: &Crate, conn: &mut AsyncPgConnection) -> QueryResult<Vec<Owner>> {
        use diesel_async::RunQueryDsl;

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

    /// Given this set of owners, determines the strongest rights the
    /// user has.
    ///
    /// Shortcircuits on `Full` because you can't beat it. In practice we'll always
    /// see `[user, user, user, ..., team, team, team]`, so we could shortcircuit on
    /// `Publish` as well, but this is a non-obvious invariant so we don't bother.
    /// Sweet free optimization if teams are proving burdensome to check.
    /// More than one team isn't really expected, though.
    pub async fn rights(&self, app: &App, owners: &[Owner]) -> AppResult<Rights> {
        let mut best = Rights::None;
        for owner in owners {
            match *owner {
                Owner::User(ref other_user) => {
                    if other_user.id == self.id {
                        return Ok(Rights::Full);
                    }
                }
                Owner::Team(ref team) => {
                    if team.contains_user(app, self).await? {
                        best = Rights::Publish;
                    }
                }
            }
        }
        Ok(best)
    }

    /// Queries the database for the verified emails
    /// belonging to a given user
    pub fn verified_email(&self, conn: &mut impl Conn) -> QueryResult<Option<String>> {
        use diesel::RunQueryDsl;

        Email::belonging_to(self)
            .select(emails::email)
            .filter(emails::verified.eq(true))
            .first(conn)
            .optional()
    }

    /// Queries the database for the verified emails
    /// belonging to a given user
    pub async fn async_verified_email(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> QueryResult<Option<String>> {
        use diesel_async::RunQueryDsl;

        Email::belonging_to(self)
            .select(emails::email)
            .filter(emails::verified.eq(true))
            .first(conn)
            .await
            .optional()
    }

    /// Queries for the email belonging to a particular user
    pub fn email(&self, conn: &mut impl Conn) -> QueryResult<Option<String>> {
        use diesel::RunQueryDsl;

        Email::belonging_to(self)
            .select(emails::email)
            .first(conn)
            .optional()
    }

    /// Queries for the email belonging to a particular user
    pub async fn async_email(&self, conn: &mut AsyncPgConnection) -> QueryResult<Option<String>> {
        use diesel_async::RunQueryDsl;

        Email::belonging_to(self)
            .select(emails::email)
            .first(conn)
            .await
            .optional()
    }
}

/// Represents a new user record insertable to the `users` table
#[derive(Insertable, Debug, Default)]
#[diesel(table_name = users, check_for_backend(diesel::pg::Pg))]
pub struct NewUser<'a> {
    pub gh_id: i32,
    pub gh_login: &'a str,
    pub name: Option<&'a str>,
    pub gh_avatar: Option<&'a str>,
    pub gh_access_token: &'a str,
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
            gh_access_token,
        }
    }

    /// Inserts the user into the database, or updates an existing one.
    pub async fn create_or_update(
        &self,
        email: Option<&'a str>,
        emails: &Emails,
        conn: &mut AsyncPgConnection,
    ) -> QueryResult<User> {
        use diesel::dsl::sql;
        use diesel::insert_into;
        use diesel::pg::upsert::excluded;
        use diesel::sql_types::Integer;
        use diesel_async::RunQueryDsl;

        conn.transaction(|conn| {
            async move {
                let user: User = insert_into(users::table)
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
                    .await?;

                // To send the user an account verification email
                if let Some(user_email) = email {
                    let new_email = NewEmail {
                        user_id: user.id,
                        email: user_email,
                    };

                    let token = insert_into(emails::table)
                        .values(&new_email)
                        .on_conflict_do_nothing()
                        .returning(emails::token)
                        .get_result::<String>(conn)
                        .await
                        .optional()?
                        .map(SecretString::from);

                    if let Some(token) = token {
                        // Swallows any error. Some users might insert an invalid email address here.
                        let email = UserConfirmEmail {
                            user_name: &user.gh_login,
                            domain: &emails.domain,
                            token,
                        };
                        let _ = emails.async_send(user_email, email).await;
                    }
                }

                Ok(user)
            }
            .scope_boxed()
        })
        .await
    }
}
