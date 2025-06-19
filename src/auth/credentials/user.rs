use crate::auth::{
    ApiTokenCredentials, ApiTokenCredentialsError, AuthorizedUser, CookieCredentials,
    CookieCredentialsError, Permission,
};
use crate::controllers;
use crate::util::errors::{BoxedAppError, forbidden};
use axum::extract::FromRequestParts;
use crates_io_database::models::ApiToken;
use diesel_async::AsyncPgConnection;
use http::header;
use http::request::Parts;

#[derive(Debug)]
pub enum UserCredentials {
    Cookie(CookieCredentials),
    ApiToken(ApiTokenCredentials),
}

impl UserCredentials {
    pub fn from_request_parts(parts: &Parts) -> Result<Self, UserCredentialsError> {
        if let Some(credentials) = CookieCredentials::from_request_parts(parts)? {
            return Ok(credentials.into());
        }

        let Some(header) = parts.headers.get(header::AUTHORIZATION) else {
            return Err(UserCredentialsError::AuthenticationRequired);
        };

        let credentials = ApiTokenCredentials::from_header(header)?;

        Ok(credentials.into())
    }

    pub async fn validate(
        &self,
        conn: &mut AsyncPgConnection,
        parts: &Parts,
        permission: Permission<'_>,
    ) -> Result<AuthorizedUser<Option<ApiToken>>, BoxedAppError> {
        match self {
            UserCredentials::Cookie(credentials) => {
                Ok(credentials.validate(conn, parts, permission).await?.into())
            }
            UserCredentials::ApiToken(credentials) => {
                Ok(credentials.validate(conn, parts, permission).await?.into())
            }
        }
    }
}

impl From<CookieCredentials> for UserCredentials {
    fn from(credentials: CookieCredentials) -> Self {
        UserCredentials::Cookie(credentials)
    }
}

impl From<ApiTokenCredentials> for UserCredentials {
    fn from(credentials: ApiTokenCredentials) -> Self {
        UserCredentials::ApiToken(credentials)
    }
}

impl<S: Send + Sync> FromRequestParts<S> for UserCredentials {
    type Rejection = BoxedAppError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        controllers::util::verify_origin(parts)?;
        Ok(Self::from_request_parts(parts)?)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UserCredentialsError {
    #[error(transparent)]
    InvalidCookieCredentials(#[from] CookieCredentialsError),
    #[error(transparent)]
    InvalidApiTokenCredentials(#[from] ApiTokenCredentialsError),
    #[error("Authentication required")]
    AuthenticationRequired,
}

impl From<UserCredentialsError> for BoxedAppError {
    fn from(err: UserCredentialsError) -> Self {
        match err {
            UserCredentialsError::InvalidCookieCredentials(err) => err.into(),
            UserCredentialsError::InvalidApiTokenCredentials(err) => err.into(),
            UserCredentialsError::AuthenticationRequired => {
                forbidden("this action requires authentication")
            }
        }
    }
}
