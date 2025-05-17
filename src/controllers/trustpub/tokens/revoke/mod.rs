use crate::app::AppState;
use crate::util::errors::{AppResult, custom};
use crates_io_database::schema::trustpub_tokens;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::{HeaderMap, StatusCode, header};
use sha2::{Digest, Sha256};

/// Revoke a temporary access token.
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

    if !bearer.starts_with(b"crates.io/oidc/") {
        let message = "Invalid authorization header";
        return Err(custom(StatusCode::UNAUTHORIZED, message));
    }

    let hashed_token = Sha256::digest(bearer);

    let mut conn = app.db_write().await?;

    diesel::delete(trustpub_tokens::table)
        .filter(trustpub_tokens::hashed_token.eq(hashed_token.as_slice()))
        .execute(&mut conn)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
