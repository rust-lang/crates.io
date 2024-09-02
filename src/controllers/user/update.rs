use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::controllers::helpers::ok_true;
use crate::models::{Email, NewEmail};
use crate::schema::emails;
use crate::tasks::spawn_blocking;
use crate::util::errors::{bad_request, server_error, AppResult};
use axum::extract::Path;
use axum::response::Response;
use axum::Json;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use http::request::Parts;
use lettre::Address;
use secrecy::{ExposeSecret, SecretString};

#[derive(Deserialize)]
pub struct UserUpdate {
    user: User,
}

#[derive(Deserialize)]
pub struct User {
    email: Option<String>,
}

/// Handles the `PUT /users/:user_id` route.
pub async fn update_user(
    state: AppState,
    Path(param_user_id): Path<i32>,
    req: Parts,
    Json(user_update): Json<UserUpdate>,
) -> AppResult<Response> {
    let conn = state.db_write().await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        let auth = AuthCheck::default().check(&req, conn)?;
        let user = auth.user();

        // need to check if current user matches user to be updated
        if user.id != param_user_id {
            return Err(bad_request("current user does not match requested user"));
        }

        let user_email = match &user_update.user.email {
            Some(email) => email.trim(),
            None => return Err(bad_request("empty email rejected")),
        };

        if user_email.is_empty() {
            return Err(bad_request("empty email rejected"));
        }

        user_email
            .parse::<Address>()
            .map_err(|_| bad_request("invalid email address"))?;

        let new_email = NewEmail {
            user_id: user.id,
            email: user_email,
        };

        let token = diesel::insert_into(emails::table)
            .values(&new_email)
            .on_conflict(emails::user_id)
            .do_update()
            .set(&new_email)
            .returning(emails::token)
            .get_result(conn)
            .map(SecretString::new)
            .map_err(|_| server_error("Error in creating token"))?;

        // This swallows any errors that occur while attempting to send the email. Some users have
        // an invalid email set in their GitHub profile, and we should let them sign in even though
        // we're trying to silently use their invalid address during signup and can't send them an
        // email. They'll then have to provide a valid email address.
        let email = UserConfirmEmail {
            user_name: &user.gh_login,
            domain: &state.emails.domain,
            token,
        };

        let _ = state.emails.send(user_email, email);

        ok_true()
    })
    .await
}

/// Handles `PUT /user/:user_id/resend` route
pub async fn regenerate_token_and_send(
    state: AppState,
    Path(param_user_id): Path<i32>,
    req: Parts,
) -> AppResult<Response> {
    let conn = state.db_write().await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        let auth = AuthCheck::default().check(&req, conn)?;
        let user = auth.user();

        // need to check if current user matches user to be updated
        if user.id != param_user_id {
            return Err(bad_request("current user does not match requested user"));
        }

        conn.transaction(|conn| -> AppResult<_> {
            let email: Email = diesel::update(Email::belonging_to(user))
                .set(emails::token.eq(sql("DEFAULT")))
                .get_result(conn)
                .optional()?
                .ok_or_else(|| bad_request("Email could not be found"))?;

            let email1 = UserConfirmEmail {
                user_name: &user.gh_login,
                domain: &state.emails.domain,
                token: email.token,
            };

            state.emails.send(&email.email, email1).map_err(Into::into)
        })?;

        ok_true()
    })
    .await
}

pub struct UserConfirmEmail<'a> {
    pub user_name: &'a str,
    pub domain: &'a str,
    pub token: SecretString,
}

impl crate::email::Email for UserConfirmEmail<'_> {
    fn subject(&self) -> String {
        "crates.io: Please confirm your email address".into()
    }

    fn body(&self) -> String {
        // Create a URL with token string as path to send to user
        // If user clicks on path, look email/user up in database,
        // make sure tokens match

        format!(
            "Hello {user_name}! Welcome to crates.io. Please click the
link below to verify your email address. Thank you!\n
https://{domain}/confirm/{token}",
            user_name = self.user_name,
            domain = self.domain,
            token = self.token.expose_secret(),
        )
    }
}
