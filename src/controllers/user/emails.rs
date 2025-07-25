use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::controllers::helpers::OkResponse;
use crate::email::EmailMessage;
use crate::models::{Email, NewEmail};
use crate::util::errors::{AppResult, bad_request, not_found, server_error};
use crate::views::EncodableEmail;
use axum::Json;
use axum::extract::{FromRequest, Path};
use crates_io_database::schema::emails;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::request::Parts;
use lettre::Address;
use minijinja::context;
use secrecy::ExposeSecret;
use serde::Deserialize;

#[derive(Deserialize, FromRequest, utoipa::ToSchema)]
#[from_request(via(Json))]
pub struct EmailCreate {
    email: String,
}

/// Add a new email address to a user profile.
#[utoipa::path(
    post,
    path = "/api/v1/users/{id}/emails",
    params(
        ("id" = i32, Path, description = "ID of the user"),
    ),
    request_body = inline(EmailCreate),
    security(
        ("api_token" = []),
        ("cookie" = []),
    ),
    tag = "users",
    responses((status = 200, description = "Successful Response", body = EncodableEmail)),
)]
pub async fn create_email(
    state: AppState,
    Path(param_user_id): Path<i32>,
    req: Parts,
    email: EmailCreate,
) -> AppResult<Json<EncodableEmail>> {
    let mut conn = state.db_write().await?;
    let auth = AuthCheck::default().check(&req, &mut conn).await?;

    // need to check if current user matches user to be updated
    if auth.user_id() != param_user_id {
        return Err(bad_request("current user does not match requested user"));
    }

    let user_email = email.email.trim();

    if user_email.is_empty() {
        return Err(bad_request("empty email rejected"));
    }

    user_email
        .parse::<Address>()
        .map_err(|_| bad_request("invalid email address"))?;

    // fetch count of user's current emails to determine if we need to mark the new email as primary
    let email_count: i64 = Email::belonging_to(&auth.user())
        .count()
        .get_result(&mut conn)
        .await
        .map_err(|_| server_error("Error fetching existing emails"))?;

    let saved_email = NewEmail::builder()
        .user_id(auth.user().id)
        .email(user_email)
        .primary(email_count == 0) // Mark as primary if this is the first email
        .build()
        .insert_if_missing(&mut conn)
        .await
        .map_err(|e| server_error(format!("{e}")))?;

    let saved_email = match saved_email {
        Some(email) => email,
        None => return Err(bad_request("email already exists")),
    };

    let verification_message = EmailMessage::from_template(
        "user_confirm",
        context! {
            user_name => auth.user().gh_login,
            domain => state.emails.domain,
            token => saved_email.token.expose_secret()
        },
    )
    .map_err(|_| server_error("Failed to render email template"))?;

    state
        .emails
        .send(&saved_email.email, verification_message)
        .await?;

    Ok(Json(EncodableEmail::from(saved_email)))
}

/// Delete an email address from a user profile.
#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}/emails/{email_id}",
    params(
        ("id" = i32, Path, description = "ID of the user"),
        ("email_id" = i32, Path, description = "ID of the email to delete"),
    ),
    security(
        ("api_token" = []),
        ("cookie" = []),
    ),
    tag = "users",
    responses((status = 200, description = "Successful Response", body = inline(OkResponse))),
)]
pub async fn delete_email(
    state: AppState,
    Path((param_user_id, email_id)): Path<(i32, i32)>,
    req: Parts,
) -> AppResult<OkResponse> {
    let mut conn = state.db_write().await?;
    let auth = AuthCheck::default().check(&req, &mut conn).await?;

    // need to check if current user matches user to be updated
    if auth.user_id() != param_user_id {
        return Err(bad_request("current user does not match requested user"));
    }

    let email = Email::belonging_to(&auth.user())
        .filter(emails::id.eq(email_id))
        .select(Email::as_select())
        .get_result(&mut conn)
        .await
        .map_err(|_| not_found())?;

    if email.primary {
        return Err(bad_request(
            "cannot delete primary email, please set another email as primary first",
        ));
    }

    diesel::delete(&email)
        .execute(&mut conn)
        .await
        .map_err(|_| server_error("Error in deleting email"))?;

    Ok(OkResponse::new())
}

/// Mark a specific email address as the primary email. This will cause notifications to be sent to this email address.
#[utoipa::path(
    put,
    path = "/api/v1/users/{id}/emails/{email_id}/set_primary",
    params(
        ("id" = i32, Path, description = "ID of the user"),
        ("email_id" = i32, Path, description = "ID of the email to set as primary"),
    ),
    security(
        ("api_token" = []),
        ("cookie" = []),
    ),
    tag = "users",
    responses((status = 200, description = "Successful Response", body = inline(OkResponse))),
)]
pub async fn set_primary_email(
    state: AppState,
    Path((param_user_id, email_id)): Path<(i32, i32)>,
    req: Parts,
) -> AppResult<OkResponse> {
    let mut conn = state.db_write().await?;
    let auth = AuthCheck::default().check(&req, &mut conn).await?;

    // need to check if current user matches user to be updated
    if auth.user_id() != param_user_id {
        return Err(bad_request("current user does not match requested user"));
    }

    let email = Email::belonging_to(&auth.user())
        .filter(emails::id.eq(email_id))
        .select(Email::as_select())
        .get_result(&mut conn)
        .await
        .map_err(|_| not_found())?;

    if email.primary {
        return Err(bad_request("email is already primary"));
    }

    diesel::sql_query("SELECT mark_email_as_primary($1)")
        .bind::<diesel::sql_types::Integer, _>(email_id)
        .execute(&mut conn)
        .await
        .map_err(|_| server_error("Error in marking email as primary"))?;

    Ok(OkResponse::new())
}
