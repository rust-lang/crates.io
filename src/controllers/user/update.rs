use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::controllers::helpers::OkResponse;
use crate::email::EmailMessage;
use crate::models::NewEmail;
use crate::schema::{abandoned_usernames, reserved_usernames, users};
use crate::util::errors::{AppResult, bad_request, server_error};

use axum::Json;
use axum::extract::Path;
use chrono::{DateTime, SecondsFormat, TimeDelta, Utc};
use crates_io_database::fns::canon_username;
use crates_io_database::models::NewAbandonedUsername;
use crates_io_validation::validate_username;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use http::request::Parts;
use lettre::Address;
use minijinja::context;
use secrecy::ExposeSecret;
use serde::Deserialize;
use tracing::warn;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UserUpdate {
    #[schema(inline)]
    user: User,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct User {
    username: Option<String>,
    email: Option<String>,
    publish_notifications: Option<bool>,
}

/// Amount of time after a username is given up before it can be re-used
const USERNAME_COOLDOWN: TimeDelta = TimeDelta::days(30);

/// Update user settings.
///
/// This endpoint allows users to update their email address and publish notifications settings.
///
/// The `id` parameter needs to match the ID of the currently authenticated user.
#[utoipa::path(
    put,
    path = "/api/v1/users/{user}",
    params(
        ("user" = i32, Path, description = "ID of the user"),
    ),
    request_body = inline(UserUpdate),
    security(
        ("api_token" = []),
        ("cookie" = []),
    ),
    tag = "users",
    responses((status = 200, description = "Successful Response", body = inline(OkResponse))),
)]
pub async fn update_user(
    state: AppState,
    Path(param_user_id): Path<i32>,
    req: Parts,
    Json(user_update): Json<UserUpdate>,
) -> AppResult<OkResponse> {
    let mut conn = state.db_write().await?;
    let auth = AuthCheck::default().check(&req, &mut conn).await?;

    let user = auth.user();

    // need to check if current user matches user to be updated
    if user.id != param_user_id {
        return Err(bad_request("current user does not match requested user"));
    }

    if let Some(publish_notifications) = &user_update.user.publish_notifications
        && user.publish_notifications != *publish_notifications
    {
        diesel::update(user)
            .set(users::publish_notifications.eq(*publish_notifications))
            .execute(&mut conn)
            .await?;

        if !publish_notifications {
            let email_address = user.verified_email(&conn).await?;

            if let Some(email_address) = email_address {
                let email = EmailMessage::from_template(
                    "unsubscribe_notifications",
                    context! {
                        user_name => user.gh_login,
                        domain => state.emails.domain
                    },
                );

                match email {
                    Ok(email) => {
                        if let Err(error) = state.emails.send(&email_address, email).await {
                            warn!(
                                "Failed to send publish notifications unsubscribe email to {email_address}: {error}"
                            );
                        }
                    }
                    Err(error) => warn!("Failed to render unsubscribe email template: {error}"),
                }
            }
        }
    }

    if let Some(newname) = &user_update.user.username
        && newname != &user.username
    {
        // stop immediately if username is invalid
        validate_username(newname).map_err(bad_request)?;

        conn.transaction(async |conn|
            {
                // checks to ensure the new username is available
                if is_reserved_username(newname, conn).await? {
                    return Err(bad_request(format!("the username `{newname}` is reserved")));
                }
                if username_conflict(newname, user.id, conn).await?
                {
                    return Err(bad_request(format!(
                        "the username `{newname}` is not available"
                    )));
                }
                if let Some(available_at) = username_has_cooldown(newname, conn).await? {
                    return Err(bad_request(format!(
                        "The username `{}` was recently in use. This username will be available after {}.",
                        newname,
                        available_at.to_rfc3339_opts(SecondsFormat::Secs, true)
                    )));
                }

                // build and apply updates to both `users` and `abandoned_usernames` tables
                let now = Utc::now();
                let available_at = now + USERNAME_COOLDOWN;
                let abandonment_record = NewAbandonedUsername {  username: &user.username,
                    previous_user_id: Some(user.id),
                    adopted_at: user.current_username_adopted_at.as_ref(),
                    abandoned_at: &now,
                    available_at: &available_at
                };

                diesel::update(user)
                    .set((users::username.eq(newname), users::current_username_adopted_at.eq(now)))
                    .execute(conn)
                    .await?;
                diesel::insert_into(abandoned_usernames::table)
                    .values(abandonment_record)
                    .execute(conn)
                    .await?;

                Ok(())
            }).await?;
    }

    if let Some(user_email) = &user_update.user.email {
        let user_email = user_email.trim();

        if user_email.is_empty() {
            return Err(bad_request("empty email rejected"));
        }

        user_email
            .parse::<Address>()
            .map_err(|_| bad_request("invalid email address"))?;

        let new_email = NewEmail::builder()
            .user_id(user.id)
            .email(user_email)
            .build();

        let token = new_email.insert_or_update(&conn).await;
        let token = token.map_err(|_| server_error("Error in creating token"))?;

        // This swallows any errors that occur while attempting to send the email. Some users have
        // an invalid email set in their GitHub profile, and we should let them sign in even though
        // we're trying to silently use their invalid address during signup and can't send them an
        // email. They'll then have to provide a valid email address.
        let email = EmailMessage::from_template(
            "user_confirm",
            context! {
                user_name => user.gh_login,
                domain => state.emails.domain,
                token => token.expose_secret()
            },
        );

        match email {
            Ok(email) => {
                let _ = state.emails.send(user_email, email).await;
            }
            Err(error) => {
                warn!("Failed to render user confirmation email template: {error}");
            }
        };
    }

    Ok(OkResponse::new())
}

/// Returns true if any users *besides the current one*
/// have a username that conflicts with this one
async fn username_conflict(
    username: &str,
    user_id: i32,
    conn: &mut AsyncPgConnection,
) -> Result<bool, diesel::result::Error> {
    let in_use_name_query: Option<String> = users::table
        .filter(users::id.ne(user_id))
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
