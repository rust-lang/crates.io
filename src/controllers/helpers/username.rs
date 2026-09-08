use crates_io_database::fns::canon_username;
use crates_io_database::schema::{abandoned_usernames, reserved_usernames, users};
use crates_io_validation::{InvalidUsername, validate_username};

use crate::util::errors::{BoxedAppError, bad_request};

use chrono::{DateTime, SecondsFormat, TimeDelta, Utc};
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

/// Amount of time after a username is given up before it can be re-used
pub(crate) const USERNAME_COOLDOWN: TimeDelta = TimeDelta::days(30);

/// Errors arising from a username validation.
///
/// Note this includes both business logic issues
/// (e.g., invalid strings) *and* runtime issues (db errors).
#[non_exhaustive]
#[derive(PartialEq, Debug, thiserror::Error)]
pub(crate) enum UsernameErr {
    #[error(transparent)]
    Diesel(#[from] DieselError),

    #[error(transparent)]
    Invalid(#[from] InvalidUsername),

    #[error("the username `{0}` is reserved")]
    Reserved(String),

    #[error("the username `{0}` is not available")]
    InUse(String),

    #[error(
        "The username `{0}` was recently in use. This username will be available after {date}.",
        date=.1.to_rfc3339_opts(SecondsFormat::Secs, true)
    )]
    Cooldown(String, DateTime<Utc>),
}

impl From<UsernameErr> for BoxedAppError {
    fn from(username_err: UsernameErr) -> Self {
        match username_err {
            UsernameErr::Diesel(err) => err.into(),
            other => bad_request(other),
        }
    }
}

/// Checks whether a username is valid and available to be adopted.
/// Should be called from the same transaction that will be used
/// to update the username.
pub(crate) async fn check_username(
    newname: &str,
    conn: &mut AsyncPgConnection,
) -> Result<(), UsernameErr> {
    if let Err(validation_err) = validate_username(newname) {
        return Err(UsernameErr::Invalid(validation_err));
    }
    if is_reserved_username(newname, conn).await? {
        return Err(UsernameErr::Reserved(newname.to_owned()));
    }
    if username_conflict(newname, conn).await? {
        return Err(UsernameErr::InUse(newname.to_owned()));
    }
    if let Some(available_at) = username_has_cooldown(newname, conn).await? {
        return Err(UsernameErr::Cooldown(newname.to_owned(), available_at));
    }

    Ok(())
}

/// Returns whether username appears in the `reserved_usernames` table.
async fn is_reserved_username(
    username: &str,
    mut conn: &mut AsyncPgConnection,
) -> Result<bool, diesel::result::Error> {
    let reserved_name_query: Option<String> = reserved_usernames::table
        .filter(canon_username(reserved_usernames::username).eq(canon_username(username)))
        .select(reserved_usernames::username)
        .first(&mut conn)
        .await
        .optional()?;

    if let Some(_reserved_name) = reserved_name_query {
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Returns whether this username is currently in use
async fn username_conflict(
    username: &str,
    conn: &mut AsyncPgConnection,
) -> Result<bool, diesel::result::Error> {
    let in_use_name_query: Option<String> = users::table
        .filter(canon_username(users::username).eq(canon_username(username)))
        .select(users::username)
        .first(conn)
        .await
        .optional()?;
    if let Some(_in_use_name) = in_use_name_query {
        Ok(true)
    } else {
        Ok(false)
    }
}

/// If username was recently abandoned and can't yet be adopted,
/// returns the UTC datetime when it will become available.
async fn username_has_cooldown(
    username: &str,
    conn: &mut AsyncPgConnection,
) -> Result<Option<DateTime<Utc>>, diesel::result::Error> {
    let abandoned_name_query: Option<(String, DateTime<Utc>)> = abandoned_usernames::table
        .filter(canon_username(abandoned_usernames::username).eq(canon_username(username)))
        .filter(abandoned_usernames::available_at.gt(Utc::now()))
        .select((
            abandoned_usernames::username,
            abandoned_usernames::available_at,
        ))
        .first(conn)
        .await
        .optional()?;

    if let Some((_abandoned_name, available_at)) = abandoned_name_query {
        Ok(Some(available_at))
    } else {
        Ok(None)
    }
}
