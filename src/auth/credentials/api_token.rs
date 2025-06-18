use crate::auth::{AuthorizedUser, Permission};
use crate::controllers;
use crate::middleware::log_request::RequestLogExt;
use crate::util::errors::{
    BoxedAppError, InsecurelyGeneratedTokenRevoked, bad_request, custom, forbidden, internal,
};
use axum::extract::FromRequestParts;
use crates_io_database::models::{ApiToken, User};
use crates_io_database::utils::token::{HashedToken, InvalidTokenError};
use diesel_async::AsyncPgConnection;
use http::header::ToStrError;
use http::request::Parts;
use http::{HeaderValue, StatusCode};

#[derive(Debug)]
pub struct ApiTokenCredentials {
    hashed_token: HashedToken,
}

impl ApiTokenCredentials {
    pub fn from_request_parts(parts: &Parts) -> Result<Self, ApiTokenCredentialsError> {
        use ApiTokenCredentialsError::*;
        use http::header;

        let header = parts.headers.get(header::AUTHORIZATION);
        let header = header.ok_or(MissingAuthorizationHeader)?;

        Self::from_header(header)
    }

    pub fn from_header(header: &HeaderValue) -> Result<Self, ApiTokenCredentialsError> {
        let header = header.to_str()?;

        let (scheme, token) = header.split_once(' ').unwrap_or(("", header));
        if !(scheme.is_empty() || scheme.eq_ignore_ascii_case("Bearer")) {
            return Err(ApiTokenCredentialsError::InvalidAuthScheme);
        }

        Self::from_raw_token(token.trim_ascii())
    }

    pub fn from_raw_token(token: &str) -> Result<Self, ApiTokenCredentialsError> {
        let hashed_token = HashedToken::parse(token)?;
        Ok(Self { hashed_token })
    }

    pub async fn validate(
        &self,
        conn: &mut AsyncPgConnection,
        parts: &Parts,
        permission: Permission<'_>,
    ) -> Result<AuthorizedUser<ApiToken>, BoxedAppError> {
        let api_token = ApiToken::find_by_api_token(conn, &self.hashed_token)
            .await
            .map_err(|e| {
                let cause = format!("invalid token caused by {e}");
                parts.request_log().add("cause", cause);
                forbidden("authentication failed")
            })?;

        let user = User::find(conn, api_token.user_id).await.map_err(|err| {
            parts.request_log().add("cause", err);
            internal("user_id from token not found in database")
        })?;

        parts.request_log().add("uid", api_token.user_id);
        parts.request_log().add("tokenid", api_token.id);

        AuthorizedUser::new(user, api_token)
            .validate(conn, parts, permission)
            .await
    }
}

impl<S: Send + Sync> FromRequestParts<S> for ApiTokenCredentials {
    type Rejection = BoxedAppError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        controllers::util::verify_origin(parts)?;
        Ok(Self::from_request_parts(parts)?)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ApiTokenCredentialsError {
    #[error("Missing `Authorization` header")]
    MissingAuthorizationHeader,
    #[error("Unexpected non-ASCII characters in `Authorization` header: {0}")]
    InvalidCharacters(#[from] ToStrError),
    #[error("Unexpected `Authorization` header scheme")]
    InvalidAuthScheme,
    #[error("Invalid API token: {0}")]
    InvalidAccessToken(#[from] InvalidTokenError),
}

impl From<ApiTokenCredentialsError> for BoxedAppError {
    fn from(err: ApiTokenCredentialsError) -> Self {
        if matches!(err, ApiTokenCredentialsError::MissingAuthorizationHeader) {
            bad_request("token not provided")
        } else if matches!(err, ApiTokenCredentialsError::InvalidAccessToken(_)) {
            InsecurelyGeneratedTokenRevoked::boxed()
        } else {
            let message = format!("Authentication failed: {err}");
            custom(StatusCode::UNAUTHORIZED, message)
        }
    }
}
