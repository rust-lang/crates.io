use crate::auth::{AuthorizedUser, Permission};
use crate::controllers;
use crate::middleware::log_request::RequestLogExt;
use crate::util::errors::{BoxedAppError, custom, forbidden, internal};
use axum::extract::FromRequestParts;
use crates_io_database::models::User;
use crates_io_session::SessionExtension;
use diesel_async::AsyncPgConnection;
use http::request::Parts;
use http::{StatusCode, header};
use std::num::ParseIntError;

#[derive(Debug, Clone, Copy)]
pub struct CookieCredentials {
    user_id: i32,
}

impl CookieCredentials {
    pub fn new(user_id: i32) -> Self {
        Self { user_id }
    }

    pub fn from_request_parts(parts: &Parts) -> Result<Option<Self>, CookieCredentialsError> {
        let Some(session) = parts.extensions.get::<SessionExtension>() else {
            error!("No `SessionExtension` found in request parts!");
            return Ok(None);
        };

        let Some(user_id) = session.get("user_id") else {
            return Ok(None);
        };

        let user_id = user_id.parse()?;

        Ok(Some(Self { user_id }))
    }

    pub async fn validate(
        &self,
        conn: &mut AsyncPgConnection,
        parts: &Parts,
        permission: Permission<'_>,
    ) -> Result<AuthorizedUser<()>, BoxedAppError> {
        let user = User::find(conn, self.user_id).await.map_err(|err| {
            parts.request_log().add("cause", err);
            internal("user_id from cookie not found in database")
        })?;

        parts.request_log().add("uid", self.user_id);

        AuthorizedUser::new(user, ())
            .validate(conn, parts, permission)
            .await
    }
}

impl<S: Send + Sync> FromRequestParts<S> for CookieCredentials {
    type Rejection = BoxedAppError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        controllers::util::verify_origin(parts)?;

        Self::from_request_parts(parts)?.ok_or_else(|| {
            if parts.headers.get(header::AUTHORIZATION).is_some() {
                forbidden("this action can only be performed on the crates.io website")
            } else {
                forbidden("this action requires authentication")
            }
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CookieCredentialsError {
    #[error("Authentication failed: Unexpected characters in `user_id` session value: {0}")]
    InvalidCharacters(#[from] ParseIntError),
}

impl From<CookieCredentialsError> for BoxedAppError {
    fn from(err: CookieCredentialsError) -> Self {
        custom(StatusCode::UNAUTHORIZED, err.to_string())
    }
}
