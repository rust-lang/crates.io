use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::controllers::helpers::ok_true;
use crate::models::NewEmail;
use crate::schema::{emails, users};
use crate::util::diesel::prelude::*;
use crate::util::errors::{bad_request, server_error, AppResult};
use axum::extract::Path;
use axum::response::Response;
use axum::Json;
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
    publish_notifications: Option<bool>,
}

/// Handles the `PUT /users/:user_id` route.
pub async fn update_user(
    state: AppState,
    Path(param_user_id): Path<i32>,
    req: Parts,
    Json(user_update): Json<UserUpdate>,
) -> AppResult<Response> {
    use diesel_async::RunQueryDsl;

    let mut conn = state.db_write().await?;
    let auth = AuthCheck::default().check(&req, &mut conn).await?;

    let user = auth.user();

    // need to check if current user matches user to be updated
    if user.id != param_user_id {
        return Err(bad_request("current user does not match requested user"));
    }

    if let Some(publish_notifications) = &user_update.user.publish_notifications {
        if user.publish_notifications != *publish_notifications {
            diesel::update(user)
                .set(users::publish_notifications.eq(*publish_notifications))
                .execute(&mut conn)
                .await?;

            if !publish_notifications {
                let email_address = user.async_verified_email(&mut conn).await?;

                if let Some(email_address) = email_address {
                    let email = PublishNotificationsUnsubscribeEmail {
                        user_name: &user.gh_login,
                        domain: &state.emails.domain,
                    };

                    if let Err(error) = state.emails.async_send(&email_address, email).await {
                        warn!("Failed to send publish notifications unsubscribe email to {email_address}: {error}");
                    }
                }
            }
        }
    }

    if let Some(user_email) = &user_update.user.email {
        let user_email = user_email.trim();

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
            .get_result::<String>(&mut conn)
            .await
            .map(SecretString::from)
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

        let _ = state.emails.async_send(user_email, email).await;
    }

    ok_true()
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

pub struct PublishNotificationsUnsubscribeEmail<'a> {
    pub user_name: &'a str,
    pub domain: &'a str,
}

impl crate::email::Email for PublishNotificationsUnsubscribeEmail<'_> {
    fn subject(&self) -> String {
        "crates.io: Unsubscribed from publish notifications".into()
    }

    fn body(&self) -> String {
        let Self { user_name, domain } = self;
        format!(
            "Hello {user_name}!

You have been unsubscribed from publish notifications.

If you would like to resubscribe, please visit https://{domain}/settings/profile",
        )
    }
}
