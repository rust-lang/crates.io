use crate::app::AppState;
use crate::auth::TrustPubCredentials;
use crate::util::errors::AppResult;
use crates_io_database::schema::trustpub_tokens;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::StatusCode;

#[cfg(test)]
mod tests;

/// Revoke a temporary access token.
///
/// The access token is expected to be passed in the `Authorization` header
/// as a `Bearer` token, similar to how it is used in the publish endpoint.
#[utoipa::path(
    delete,
    path = "/api/v1/trusted_publishing/tokens",
    security(("trustpub_token" = [])),
    tag = "trusted_publishing",
    responses((status = 204, description = "Successful Response")),
)]
pub async fn revoke_trustpub_token(
    app: AppState,
    creds: TrustPubCredentials,
) -> AppResult<StatusCode> {
    let token = creds.unvalidated_token();
    let hashed_token = token.sha256();

    let mut conn = app.db_write().await?;

    diesel::delete(trustpub_tokens::table)
        .filter(trustpub_tokens::hashed_token.eq(hashed_token.as_slice()))
        .execute(&mut conn)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
