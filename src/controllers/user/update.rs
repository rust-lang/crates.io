use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::controllers::helpers::OkResponse;
use crate::email::EmailMessage;
use crate::models::{Email, NewEmail};
use crate::schema::users;
use crate::util::errors::{AppResult, bad_request, server_error};
use axum::Json;
use axum::extract::Path;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::request::Parts;
use lettre::Address;
use minijinja::context;
use secrecy::ExposeSecret;
use serde::Deserialize;
use tracing::warn;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UserUpdate {
    user: User,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[schema(as = UserUpdateParameters)]
pub struct User {
    #[deprecated(note = "Use `/api/v1/users/{id}/emails` instead.")]
    email: Option<String>,
    publish_notifications: Option<bool>,
}

/// Update user settings.
///
/// This endpoint allows users to manage publish notifications settings.
///
/// You may provide an `email` parameter to add a new email address to the user's profile, but
/// this is for legacy support only and will be removed in the future.
///
/// For managing email addresses, please use the `/api/v1/users/{id}/emails` endpoints instead.
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

    if let Some(publish_notifications) = &user_update.user.publish_notifications {
        if user.publish_notifications != *publish_notifications {
            diesel::update(user)
                .set(users::publish_notifications.eq(*publish_notifications))
                .execute(&mut conn)
                .await?;

            if !publish_notifications {
                let email_address = user.verified_email(&mut conn).await?;

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
    }

    #[allow(deprecated)]
    if let Some(user_email) = &user_update.user.email {
        let user_email = user_email.trim();

        if user_email.is_empty() {
            return Err(bad_request("empty email rejected"));
        }

        user_email
            .parse::<Address>()
            .map_err(|_| bad_request("invalid email address"))?;

        // Check if this is the first email for the user, because if so, we need to enable notifications
        let existing_email_count: i64 = Email::belonging_to(&user)
            .count()
            .get_result(&mut conn)
            .await
            .map_err(|_| server_error("Error fetching existing emails"))?;

        let saved_email = NewEmail::builder()
            .user_id(user.id)
            .email(user_email)
            .send_notifications(existing_email_count < 1) // Enable notifications if this is the first email
            .build()
            .insert_if_missing(&mut conn)
            .await
            .map_err(|_| server_error("Error saving email"))?;

        if let Some(saved_email) = saved_email {
            // This swallows any errors that occur while attempting to send the email. Some users have
            // an invalid email set in their GitHub profile, and we should let them sign in even though
            // we're trying to silently use their invalid address during signup and can't send them an
            // email. They'll then have to provide a valid email address.
            let email = EmailMessage::from_template(
                "user_confirm",
                context! {
                    user_name => user.gh_login,
                    domain => state.emails.domain,
                    token => saved_email.token.expose_secret()
                },
            );

            match email {
                Ok(email) => {
                    let _ = state.emails.send(user_email, email).await;
                }
                Err(error) => {
                    warn!("Failed to render user confirmation email template: {error}");
                }
            }
        }
    }

    Ok(OkResponse::new())
}
