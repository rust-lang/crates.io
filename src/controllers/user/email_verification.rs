use super::update::UserConfirmEmail;
use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::controllers::helpers::OkResponse;
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

/// Regenerate and send an email verification token.
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
pub async fn resend_email_verification(
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
            let email: Email = diesel::update(Email::belonging_to(auth.user()))
                .set(emails::token.eq(sql("DEFAULT")))
                .get_result(conn)
                .await
                .optional()?
                .ok_or_else(|| bad_request("Email could not be found"))?;

            let email1 = UserConfirmEmail {
                user_name: &auth.user().gh_login,
                domain: &state.emails.domain,
                token: email.token,
            };

            state
                .emails
                .send(&email.email, email1)
                .await
                .map_err(BoxedAppError::from)
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
    async fn test_no_auth() {
        let (app, anon, user) = TestApp::init().with_user().await;

        let url = format!("/api/v1/users/{}/resend", user.as_model().id);
        let response = anon.put::<()>(&url, "").await;
        assert_snapshot!(response.status(), @"403 Forbidden");
        assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);

        assert_eq!(app.emails().await.len(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_wrong_user() {
        let (app, _anon, user) = TestApp::init().with_user().await;
        let user2 = app.db_new_user("bar").await;

        let url = format!("/api/v1/users/{}/resend", user2.as_model().id);
        let response = user.put::<()>(&url, "").await;
        assert_snapshot!(response.status(), @"400 Bad Request");
        assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"current user does not match requested user"}]}"#);

        assert_eq!(app.emails().await.len(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_happy_path() {
        let (app, _anon, user) = TestApp::init().with_user().await;

        let url = format!("/api/v1/users/{}/resend", user.as_model().id);
        let response = user.put::<()>(&url, "").await;
        assert_snapshot!(response.status(), @"200 OK");
        assert_snapshot!(response.text(), @r#"{"ok":true}"#);

        assert_snapshot!(app.emails_snapshot().await);
    }
}
