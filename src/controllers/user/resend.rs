use super::update::UserConfirmEmail;
use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::controllers::helpers::ok_true;
use crate::models::Email;
use crate::tasks::spawn_blocking;
use crate::util::errors::bad_request;
use crate::util::errors::AppResult;
use axum::extract::Path;
use axum::response::Response;
use crates_io_database::schema::emails;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use http::request::Parts;

/// Handles `PUT /user/:user_id/resend` route
pub async fn regenerate_token_and_send(
    state: AppState,
    Path(param_user_id): Path<i32>,
    req: Parts,
) -> AppResult<Response> {
    let mut conn = state.db_write().await?;
    let auth = AuthCheck::default().check(&req, &mut conn).await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

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

#[cfg(test)]
mod tests {
    use crate::tests::util::{RequestHelper, TestApp};
    use http::StatusCode;
    use insta::assert_snapshot;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_no_auth() {
        let (app, anon, user) = TestApp::init().with_user();

        let url = format!("/api/v1/users/{}/resend", user.as_model().id);
        let response = anon.put::<()>(&url, "").await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);

        assert_eq!(app.emails().len(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_wrong_user() {
        let (app, _anon, user) = TestApp::init().with_user();
        let user2 = app.db_new_user("bar");

        let url = format!("/api/v1/users/{}/resend", user2.as_model().id);
        let response = user.put::<()>(&url, "").await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"current user does not match requested user"}]}"#);

        assert_eq!(app.emails().len(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_happy_path() {
        let (app, _anon, user) = TestApp::init().with_user();

        let url = format!("/api/v1/users/{}/resend", user.as_model().id);
        let response = user.put::<()>(&url, "").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_snapshot!(response.text(), @r###"{"ok":true}"###);

        assert_snapshot!(app.emails_snapshot());
    }
}
