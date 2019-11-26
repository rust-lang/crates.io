use diesel::dsl::now;
use diesel::prelude::*;
use std::borrow::Cow;

use crate::app::App;
use crate::util::CargoResult;

use crate::models::{Crate, CrateOwner, Email, NewEmail, Owner, OwnerKind, Rights};
use crate::schema::{crate_owners, emails, users};
use crate::views::{EncodablePrivateUser, EncodablePublicUser};

/// The model representing a row in the `users` database table.
#[derive(Clone, Debug, PartialEq, Eq, Queryable, Identifiable, AsChangeset, Associations)]
pub struct User {
    pub id: i32,
    pub email: Option<String>,
    pub gh_access_token: String,
    pub gh_login: String,
    pub name: Option<String>,
    pub gh_avatar: Option<String>,
    pub gh_id: i32,
}

#[derive(Insertable, Debug, Default)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub gh_id: i32,
    pub gh_login: &'a str,
    pub email: Option<&'a str>,
    pub name: Option<&'a str>,
    pub gh_avatar: Option<&'a str>,
    pub gh_access_token: Cow<'a, str>,
}

pub type UserNoEmailType = (
    // pub id:
    i32,
    // pub email:
    // Option<String>,
    // pub gh_access_token:
    String,
    // pub gh_login:
    String,
    // pub name:
    Option<String>,
    // pub gh_avatar:
    Option<String>,
    // pub gh_id:
    i32,
);

pub type AllColumns = (
    users::id,
    users::gh_access_token,
    users::gh_login,
    users::name,
    users::gh_avatar,
    users::gh_id,
);

pub const ALL_COLUMNS: AllColumns = (
    users::id,
    users::gh_access_token,
    users::gh_login,
    users::name,
    users::gh_avatar,
    users::gh_id,
);

impl<'a> NewUser<'a> {
    pub fn new(
        gh_id: i32,
        gh_login: &'a str,
        email: Option<&'a str>,
        name: Option<&'a str>,
        gh_avatar: Option<&'a str>,
        gh_access_token: &'a str,
    ) -> Self {
        NewUser {
            gh_id,
            gh_login,
            email,
            name,
            gh_avatar,
            gh_access_token: Cow::Borrowed(gh_access_token),
        }
    }

    /// Inserts the user into the database, or updates an existing one.
    pub fn create_or_update(&self, conn: &PgConnection) -> QueryResult<User> {
        use crate::schema::users::dsl::*;
        use diesel::dsl::sql;
        use diesel::insert_into;
        use diesel::pg::upsert::excluded;
        use diesel::sql_types::Integer;

        conn.transaction(|| {
            let user = insert_into(users)
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
                .returning(ALL_COLUMNS)
                .get_result::<UserNoEmailType>(conn)?;

            // To send the user an account verification email...
            if let Some(user_email) = self.email {
                let new_email = NewEmail {
                    user_id: user.0,
                    email: user_email,
                };

                let token = insert_into(emails::table)
                    .values(&new_email)
                    .on_conflict_do_nothing()
                    .returning(emails::token)
                    .get_result::<String>(conn)
                    .optional()?;

                if let Some(token) = token {
                    crate::email::send_user_confirm_email(user_email, &user.2, &token);
                }
            }

            Ok(User::from(user))
        })
    }
}

impl User {
    /// Queries the database for a user with a certain `api_token` value.
    pub fn find_by_api_token(conn: &PgConnection, token_: &str) -> QueryResult<User> {
        use crate::schema::api_tokens::dsl::{api_tokens, last_used_at, revoked, token, user_id};
        use diesel::update;

        let tokens = api_tokens
            .filter(token.eq(token_))
            .filter(revoked.eq(false));

        // If the database is in read only mode, we can't update last_used_at.
        // Try updating in a new transaction, if that fails, fall back to reading
        let user_id_ = conn
            .transaction(|| {
                update(tokens)
                    .set(last_used_at.eq(now.nullable()))
                    .returning(user_id)
                    .get_result::<i32>(conn)
            })
            .or_else(|_| tokens.select(user_id).first(conn))?;

        users::table
            .select(ALL_COLUMNS)
            .find(user_id_)
            .first::<UserNoEmailType>(conn)
            .map(User::from)
    }

    pub fn owning(krate: &Crate, conn: &PgConnection) -> CargoResult<Vec<Owner>> {
        let users = CrateOwner::by_owner_kind(OwnerKind::User)
            .inner_join(users::table)
            .select(ALL_COLUMNS)
            .filter(crate_owners::crate_id.eq(krate.id))
            .load::<UserNoEmailType>(conn)?
            .into_iter()
            .map(User::from)
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
    pub fn rights(&self, app: &App, owners: &[Owner]) -> CargoResult<Rights> {
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

    pub fn verified_email(&self, conn: &PgConnection) -> CargoResult<Option<String>> {
        Ok(Email::belonging_to(self)
            .select(emails::email)
            .filter(emails::verified.eq(true))
            .first::<String>(&*conn)
            .optional()?)
    }

    /// Converts this `User` model into an `EncodablePrivateUser` for JSON serialization.
    pub fn encodable_private(
        self,
        email: Option<String>,
        email_verified: bool,
        email_verification_sent: bool,
    ) -> EncodablePrivateUser {
        let User {
            id,
            name,
            gh_login,
            gh_avatar,
            ..
        } = self;
        let url = format!("https://github.com/{}", gh_login);
        EncodablePrivateUser {
            id,
            email,
            email_verified,
            email_verification_sent,
            avatar: gh_avatar,
            login: gh_login,
            name,
            url: Some(url),
        }
    }

    /// Converts this`User` model into an `EncodablePublicUser` for JSON serialization.
    pub fn encodable_public(self) -> EncodablePublicUser {
        let User {
            id,
            name,
            gh_login,
            gh_avatar,
            ..
        } = self;
        let url = format!("https://github.com/{}", gh_login);
        EncodablePublicUser {
            id,
            avatar: gh_avatar,
            login: gh_login,
            name,
            url: Some(url),
        }
    }
}

impl From<UserNoEmailType> for User {
    fn from(user: UserNoEmailType) -> Self {
        User {
            id: user.0,
            email: None,
            gh_access_token: user.1,
            gh_login: user.2,
            name: user.3,
            gh_avatar: user.4,
            gh_id: user.5,
        }
    }
}
