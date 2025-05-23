use crate::app::AppState;
use crate::util::errors::{AppResult, custom};
use crates_io_database::schema::trustpub_tokens;
use crates_io_trustpub::access_token::AccessToken;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::{HeaderMap, StatusCode, header};

#[cfg(test)]
mod tests;

/// Revoke a temporary access token.
///
/// The access token is expected to be passed in the `Authorization` header
/// as a `Bearer` token, similar to how it is used in the publish endpoint.
#[utoipa::path(
    delete,
    path = "/api/v1/trusted_publishing/tokens",
    tag = "trusted_publishing",
    responses((status = 204, description = "Successful Response")),
)]
pub async fn revoke_trustpub_token(app: AppState, headers: HeaderMap) -> AppResult<StatusCode> {
    let Some(auth_header) = headers.get(header::AUTHORIZATION) else {
        let message = "Missing authorization header";
        return Err(custom(StatusCode::UNAUTHORIZED, message));
    };

    let Some(bearer) = auth_header.as_bytes().strip_prefix(b"Bearer ") else {
        let message = "Invalid authorization header";
        return Err(custom(StatusCode::UNAUTHORIZED, message));
    };

    let Ok(token) = AccessToken::from_byte_str(bearer) else {
        let message = "Invalid authorization header";
        return Err(custom(StatusCode::UNAUTHORIZED, message));
    };

    let hashed_token = token.sha256();

    let mut conn = app.db_write().await?;

    diesel::delete(trustpub_tokens::table)
        .filter(trustpub_tokens::hashed_token.eq(hashed_token.as_slice()))
        .execute(&mut conn)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
