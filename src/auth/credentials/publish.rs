use crate::auth::{
    ApiTokenCredentials, ApiTokenCredentialsError, AuthorizedEntity, CookieCredentials,
    CookieCredentialsError, Permission, TrustPubCredentials, TrustPubCredentialsError,
    UserCredentials,
};
use crate::controllers;
use crate::util::errors::{BoxedAppError, forbidden};
use axum::extract::FromRequestParts;
use crates_io_trustpub::access_token::AccessToken;
use diesel_async::AsyncPgConnection;
use http::header;
use http::request::Parts;

pub enum PublishCredentials {
    User(UserCredentials),
    TrustPub(TrustPubCredentials),
}

impl PublishCredentials {
    pub fn from_request_parts(parts: &Parts) -> Result<Self, PublishCredentialsError> {
        if let Some(credentials) = CookieCredentials::from_request_parts(parts)? {
            return Ok(credentials.into());
        }

        let Some(header) = parts.headers.get(header::AUTHORIZATION) else {
            return Err(PublishCredentialsError::AuthenticationRequired);
        };

        let header = header.to_str().map_err(ApiTokenCredentialsError::from)?;

        let (scheme, token) = header.split_once(' ').unwrap_or(("", header));
        if !(scheme.is_empty() || scheme.eq_ignore_ascii_case("Bearer")) {
            return Err(PublishCredentialsError::InvalidApiTokenCredentials(
                ApiTokenCredentialsError::InvalidAuthScheme,
            ));
        }

        let token = token.trim_ascii();
        if token.starts_with(AccessToken::PREFIX) {
            Ok(TrustPubCredentials::from_raw_token(token)?.into())
        } else {
            Ok(ApiTokenCredentials::from_raw_token(token)?.into())
        }
    }

    pub async fn validate(
        &self,
        conn: &mut AsyncPgConnection,
        parts: &Parts,
        permission: Permission<'_>,
    ) -> Result<AuthorizedEntity, BoxedAppError> {
        match self {
            PublishCredentials::User(credentials) => {
                let auth = credentials.validate(conn, parts, permission).await?;
                Ok(AuthorizedEntity::User(Box::new(auth)))
            }
            PublishCredentials::TrustPub(credentials) => {
                let auth = credentials.validate(conn, parts, permission).await?;
                Ok(AuthorizedEntity::TrustPub(auth))
            }
        }
    }
}

impl From<CookieCredentials> for PublishCredentials {
    fn from(credentials: CookieCredentials) -> Self {
        PublishCredentials::User(credentials.into())
    }
}

impl From<ApiTokenCredentials> for PublishCredentials {
    fn from(credentials: ApiTokenCredentials) -> Self {
        PublishCredentials::User(credentials.into())
    }
}

impl From<TrustPubCredentials> for PublishCredentials {
    fn from(credentials: TrustPubCredentials) -> Self {
        PublishCredentials::TrustPub(credentials)
    }
}

impl<S: Send + Sync> FromRequestParts<S> for PublishCredentials {
    type Rejection = BoxedAppError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        controllers::util::verify_origin(parts)?;
        Ok(Self::from_request_parts(parts)?)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PublishCredentialsError {
    #[error(transparent)]
    InvalidCookieCredentials(#[from] CookieCredentialsError),
    #[error(transparent)]
    InvalidApiTokenCredentials(#[from] ApiTokenCredentialsError),
    #[error(transparent)]
    InvalidTrustPubCredentials(#[from] TrustPubCredentialsError),
    #[error("Authentication required")]
    AuthenticationRequired,
}

impl From<PublishCredentialsError> for BoxedAppError {
    fn from(err: PublishCredentialsError) -> Self {
        match err {
            PublishCredentialsError::InvalidCookieCredentials(err) => err.into(),
            PublishCredentialsError::InvalidApiTokenCredentials(err) => err.into(),
            PublishCredentialsError::InvalidTrustPubCredentials(err) => err.into(),
            PublishCredentialsError::AuthenticationRequired => {
                forbidden("this action requires authentication")
            }
        }
    }
}
