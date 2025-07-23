use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::controllers::helpers::OkResponse;
use crate::email::EmailMessage;
use crate::models::Email;
use crate::util::errors::AppResult;
use crate::util::errors::{BoxedAppError, bad_request};
use axum::extract::Path;
use crates_io_database::schema::emails;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use http::request::Parts;
use minijinja::context;
use secrecy::ExposeSecret;

/// Marks the email belonging to the given token as verified.
#[utoipa::path(
    put,
    path = "/api/v1/confirm/{email_token}",
    params(
        ("email_token" = String, Path, description = "Secret verification token sent to the user's email address"),
    ),
    tag = "users",
    responses((status = 200, description = "Successful Response", body = inline(OkResponse))),
)]
pub async fn confirm_user_email(
    state: AppState,
    Path(token): Path<String>,
) -> AppResult<OkResponse> {
    let mut conn = state.db_write().await?;

    let updated_rows = diesel::update(emails::table.filter(emails::token.eq(&token)))
        .set(emails::verified.eq(true))
        .execute(&mut conn)
        .await?;

    if updated_rows == 0 {
        return Err(bad_request("Email belonging to token not found."));
    }

    Ok(OkResponse::new())
}

/// Regenerate and send an email verification token for the given email.
#[utoipa::path(
    put,
    path = "/api/v1/users/{user_id}/emails/{id}/resend",
    params(
        ("user_id" = i32, Path, description = "ID of the user"),
        ("id" = i32, Path, description = "ID of the email"),
    ),
    security(
        ("api_token" = []),
        ("cookie" = []),
    ),
    tag = "users",
    responses((status = 200, description = "Successful Response", body = inline(OkResponse))),
)]
pub async fn resend_email_verification(
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

    // Generate a new token for the email, if it exists and is unverified
    conn.transaction(|conn| {
        async move {
            let email: Email = diesel::update(Email::belonging_to(auth.user()))
                .set(emails::token.eq(sql("DEFAULT")))
                .returning(Email::as_returning())
                .get_result(conn)
                .await
                .optional()?
                .ok_or_else(|| bad_request("Email could not be found"))?;
            let email: Email = diesel::update(
                emails::table
                    .filter(emails::id.eq(email_id))
                    .filter(emails::user_id.eq(auth.user_id()))
                    .filter(emails::verified.eq(false)),
            )
            .set(emails::token.eq(sql("DEFAULT")))
            .returning(Email::as_returning())
            .get_result(conn)
            .await
            .optional()?
            .ok_or_else(|| bad_request("Email not found or already verified"))?;

            // Send the updated token via email
            let email_message = EmailMessage::from_template(
                "user_confirm",
                context! {
                    user_name => auth.user().gh_login,
                    domain => state.emails.domain,
                    token => email.token.expose_secret()
                },
            )
            .map_err(|_| bad_request("Failed to render email template"))?;

            state
                .emails
                .send(&email.email, email_message)
                .await
                .map_err(BoxedAppError::from)?;

            Ok::<(), BoxedAppError>(())
        }
        .scope_boxed()
    })
    .await?;

    Ok(OkResponse::new())
}

/// Regenerate and send an email verification token for any unverified email of the current user.
/// Deprecated endpoint, use `PUT /api/v1/user/{user_id}/emails/{id}/resend` instead.
#[utoipa::path(
    put,
    path = "/api/v1/users/{id}/resend",
    params(
        ("id" = i32, Path, description = "ID of the user"),
    ),
    security(
        ("api_token" = []),
        ("cookie" = []),
    ),
    tag = "users",
    responses((status = 200, description = "Successful Response", body = inline(OkResponse))),
)]
#[deprecated]
pub async fn resend_email_verification_all(
    state: AppState,
    Path(param_user_id): Path<i32>,
    req: Parts,
) -> AppResult<OkResponse> {
    let mut conn = state.db_write().await?;
    let auth = AuthCheck::default().check(&req, &mut conn).await?;

    // need to check if current user matches user to be updated
    if auth.user_id() != param_user_id {
        return Err(bad_request("current user does not match requested user"));
    }

    conn.transaction(|conn| {
        async move {
            let emails: Vec<Email> = diesel::update(
                emails::table
                    .filter(emails::user_id.eq(auth.user_id()))
                    .filter(emails::verified.eq(false)),
            )
            .set(emails::token.eq(sql("DEFAULT")))
            .returning(Email::as_returning())
            .get_results(conn)
            .await?;

            if emails.is_empty() {
                return Err(bad_request("No unverified emails found"));
            }

            for email in emails {
                let email_message = EmailMessage::from_template(
                    "user_confirm",
                    context! {
                        user_name => auth.user().gh_login,
                        domain => state.emails.domain,
                        token => email.token.expose_secret()
                    },
                )
                .map_err(|_| bad_request("Failed to render email template"))?;

                state
                    .emails
                    .send(&email.email, email_message)
                    .await
                    .map_err(BoxedAppError::from)?;
            }

            Ok(())
        }
        .scope_boxed()
    })
    .await?;

    Ok(OkResponse::new())
}

#[cfg(test)]
mod tests {
    use crate::tests::util::{RequestHelper, TestApp};
    use insta::assert_snapshot;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_legacy_no_auth() {
        let (app, anon, user) = TestApp::init().with_user().await;

        let url = format!("/api/v1/users/{}/resend", user.as_model().id);
        let response = anon.put::<()>(&url, "").await;
        assert_snapshot!(response.status(), @"403 Forbidden");
        assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);

        assert_eq!(app.emails().await.len(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_legacy_wrong_user() {
        let (app, _anon, user) = TestApp::init().with_user().await;
        let user2 = app.db_new_user("bar").await;

        let url = format!("/api/v1/users/{}/resend", user2.as_model().id);
        let response = user.put::<()>(&url, "").await;
        assert_snapshot!(response.status(), @"400 Bad Request");
        assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"current user does not match requested user"}]}"#);

        assert_eq!(app.emails().await.len(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_legacy_happy_path() {
        let (app, _anon, user) = TestApp::init().with_user().await;

        // Create a new email to be verified, inserting directly into the database so that verification is not sent
        let _new_email = user.db_new_email("bar@example.com", false, false).await;

        // Request a verification email
        let url = format!("/api/v1/users/{}/resend", user.as_model().id);
        let response = user.put::<()>(&url, "").await;
        assert_snapshot!(response.status(), @"200 OK");
        assert_snapshot!(response.text(), @r#"{"ok":true}"#);

        assert_snapshot!(app.emails_snapshot().await);
    }
}
