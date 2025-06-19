use crate::auth::{AuthorizedTrustPub, Permission};
use crate::controllers;
use crate::util::errors::{BoxedAppError, custom, forbidden};
use axum::extract::FromRequestParts;
use crates_io_database::schema::trustpub_tokens;
use crates_io_trustpub::access_token::{AccessToken, AccessTokenError};
use diesel::dsl::now;
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use http::StatusCode;
use http::header::ToStrError;
use http::request::Parts;

#[derive(Debug)]
pub struct TrustPubCredentials {
    token: AccessToken,
}

impl TrustPubCredentials {
    pub fn from_request_parts(parts: &Parts) -> Result<Self, TrustPubCredentialsError> {
        use TrustPubCredentialsError::*;
        use http::header;

        let header = parts.headers.get(header::AUTHORIZATION);
        let header = header.ok_or(MissingAuthorizationHeader)?;
        let header = header.to_str()?;

        let (scheme, token) = header.split_once(' ').unwrap_or(("", header));
        if !(scheme.is_empty() || scheme.eq_ignore_ascii_case("Bearer")) {
            return Err(InvalidAuthScheme);
        }

        Self::from_raw_token(token.trim_ascii())
    }

    pub fn from_raw_token(token: &str) -> Result<Self, TrustPubCredentialsError> {
        let token = token.parse::<AccessToken>()?;
        Ok(Self { token })
    }

    pub async fn validate(
        &self,
        conn: &mut AsyncPgConnection,
        _parts: &Parts,
        permission: Permission<'_>,
    ) -> Result<AuthorizedTrustPub, BoxedAppError> {
        let hashed_token = self.token.sha256();

        let crate_ids = trustpub_tokens::table
            .filter(trustpub_tokens::hashed_token.eq(hashed_token.as_slice()))
            .filter(trustpub_tokens::expires_at.gt(now))
            .select(trustpub_tokens::crate_ids)
            .get_result::<Vec<Option<i32>>>(conn)
            .await
            .optional()?
            .ok_or_else(|| forbidden("Invalid authentication token"))?;

        let crate_ids = crate_ids.into_iter().flatten().collect();

        AuthorizedTrustPub::new(crate_ids)
            .validate(permission)
            .await
    }

    pub fn unvalidated_token(&self) -> &AccessToken {
        &self.token
    }
}

impl<S: Send + Sync> FromRequestParts<S> for TrustPubCredentials {
    type Rejection = BoxedAppError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        controllers::util::verify_origin(parts)?;
        Ok(Self::from_request_parts(parts)?)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TrustPubCredentialsError {
    #[error("Missing `Authorization` header")]
    MissingAuthorizationHeader,
    #[error("Unexpected non-ASCII characters in `Authorization` header: {0}")]
    InvalidCharacters(#[from] ToStrError),
    #[error("Unexpected `Authorization` header scheme")]
    InvalidAuthScheme,
    #[error("Invalid access token: {0}")]
    InvalidAccessToken(#[from] AccessTokenError),
}

impl From<TrustPubCredentialsError> for BoxedAppError {
    fn from(err: TrustPubCredentialsError) -> Self {
        if matches!(err, TrustPubCredentialsError::InvalidAccessToken(_)) {
            let message = "Invalid `Authorization` header: Failed to parse token";
            custom(StatusCode::UNAUTHORIZED, message)
        } else if matches!(err, TrustPubCredentialsError::MissingAuthorizationHeader) {
            let message = "Missing `Authorization` header";
            custom(StatusCode::UNAUTHORIZED, message)
        } else {
            let message = format!("Authentication failed: {err}");
            custom(StatusCode::UNAUTHORIZED, message)
        }
    }
}
