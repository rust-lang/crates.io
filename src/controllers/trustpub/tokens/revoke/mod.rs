use crate::app::AppState;
use crate::auth::AuthHeader;
use crate::util::errors::{AppResult, custom};
use crates_io_database::schema::trustpub_tokens;
use crates_io_trustpub::access_token::AccessToken;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::StatusCode;
use secrecy::ExposeSecret;

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
pub async fn revoke_trustpub_token(app: AppState, auth: AuthHeader) -> AppResult<StatusCode> {
    let token = auth.token().expose_secret();
    let Ok(token) = token.parse::<AccessToken>() else {
        let message = "Invalid `Authorization` header: Failed to parse token";
        return Err(custom(StatusCode::UNAUTHORIZED, message));
    };

    let hashed_token = token.sha256();

    let mut conn = app.db_write().await?;

    #[expect(deprecated)]
    diesel::delete(trustpub_tokens::table)
        .filter(trustpub_tokens::hashed_token.eq(hashed_token.as_slice()))
        .execute(&mut conn)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
