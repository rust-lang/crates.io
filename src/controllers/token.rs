use crate::models::ApiToken;
use crate::schema::api_tokens;
use crate::util::rfc3339;
use crate::views::EncodableApiTokenWithToken;

use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::models::token::{CrateScope, EndpointScope};
use crate::util::errors::{bad_request, AppResult};
use axum::extract::{Path, Query};
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_extra::json;
use axum_extra::response::ErasedJson;
use chrono::NaiveDateTime;
use diesel::data_types::PgInterval;
use diesel::dsl::{now, IntervalDsl};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::request::Parts;
use http::StatusCode;

#[derive(Deserialize)]
pub struct GetParams {
    expired_days: Option<i32>,
}

impl GetParams {
    fn expired_days_interval(&self) -> PgInterval {
        match self.expired_days {
            Some(days) if days > 0 => days,
            _ => 0,
        }
        .days()
    }
}

/// List all API tokens of the authenticated user.
#[utoipa::path(
    get,
    path = "/api/v1/me/tokens",
    security(("cookie" = [])),
    tag = "api_tokens",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn list_api_tokens(
    app: AppState,
    Query(params): Query<GetParams>,
    req: Parts,
) -> AppResult<ErasedJson> {
    let mut conn = app.db_read_prefer_primary().await?;
    let auth = AuthCheck::only_cookie().check(&req, &mut conn).await?;
    let user = auth.user();

    let tokens: Vec<ApiToken> = ApiToken::belonging_to(user)
        .select(ApiToken::as_select())
        .filter(api_tokens::revoked.eq(false))
        .filter(
            api_tokens::expired_at.is_null().or(api_tokens::expired_at
                .assume_not_null()
                .gt(now - params.expired_days_interval())),
        )
        .order(api_tokens::id.desc())
        .load(&mut conn)
        .await?;

    Ok(json!({ "api_tokens": tokens }))
}

/// The incoming serialization format for the `ApiToken` model.
#[derive(Deserialize)]
pub struct NewApiToken {
    name: String,
    crate_scopes: Option<Vec<String>>,
    endpoint_scopes: Option<Vec<String>>,
    #[serde(default, with = "rfc3339::option")]
    expired_at: Option<NaiveDateTime>,
}

/// The incoming serialization format for the `ApiToken` model.
#[derive(Deserialize)]
pub struct NewApiTokenRequest {
    api_token: NewApiToken,
}

/// Create a new API token.
#[utoipa::path(
    put,
    path = "/api/v1/me/tokens",
    security(("cookie" = [])),
    tag = "api_tokens",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn create_api_token(
    app: AppState,
    parts: Parts,
    Json(new): Json<NewApiTokenRequest>,
) -> AppResult<ErasedJson> {
    if new.api_token.name.is_empty() {
        return Err(bad_request("name must have a value"));
    }

    let mut conn = app.db_write().await?;
    let auth = AuthCheck::default().check(&parts, &mut conn).await?;

    if auth.api_token_id().is_some() {
        return Err(bad_request(
            "cannot use an API token to create a new API token",
        ));
    }

    let user = auth.user();

    let max_token_per_user = 500;
    let count: i64 = ApiToken::belonging_to(user)
        .count()
        .get_result(&mut conn)
        .await?;
    if count >= max_token_per_user {
        return Err(bad_request(format!(
            "maximum tokens per user is: {max_token_per_user}"
        )));
    }

    let crate_scopes = new
        .api_token
        .crate_scopes
        .map(|scopes| {
            scopes
                .into_iter()
                .map(CrateScope::try_from)
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()
        .map_err(|_err| bad_request("invalid crate scope"))?;

    let endpoint_scopes = new
        .api_token
        .endpoint_scopes
        .map(|scopes| {
            scopes
                .into_iter()
                .map(|scope| EndpointScope::try_from(scope.as_bytes()))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()
        .map_err(|_err| bad_request("invalid endpoint scope"))?;

    let recipient = user.email(&mut conn).await?;

    let api_token = ApiToken::insert_with_scopes(
        &mut conn,
        user.id,
        &new.api_token.name,
        crate_scopes,
        endpoint_scopes,
        new.api_token.expired_at,
    )
    .await?;

    if let Some(recipient) = recipient {
        let email = NewTokenEmail {
            token_name: &new.api_token.name,
            user_name: &user.gh_login,
            domain: &app.emails.domain,
        };

        // At this point the token has been created so failing to send the
        // email should not cause an error response to be returned to the
        // caller.
        let email_ret = app.emails.send(&recipient, email).await;
        if let Err(e) = email_ret {
            error!("Failed to send token creation email: {e}")
        }
    }

    let api_token = EncodableApiTokenWithToken::from(api_token);

    Ok(json!({ "api_token": api_token }))
}

/// Find API token by id.
#[utoipa::path(
    get,
    path = "/api/v1/me/tokens/{id}",
    params(
        ("id" = i32, Path, description = "ID of the API token"),
    ),
    security(
        ("api_token" = []),
        ("cookie" = []),
    ),
    tag = "api_tokens",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn find_api_token(
    app: AppState,
    Path(id): Path<i32>,
    req: Parts,
) -> AppResult<ErasedJson> {
    let mut conn = app.db_write().await?;
    let auth = AuthCheck::default().check(&req, &mut conn).await?;
    let user = auth.user();
    let token = ApiToken::belonging_to(user)
        .find(id)
        .select(ApiToken::as_select())
        .first(&mut conn)
        .await?;

    Ok(json!({ "api_token": token }))
}

/// Revoke API token.
#[utoipa::path(
    delete,
    path = "/api/v1/me/tokens/{id}",
    params(
        ("id" = i32, Path, description = "ID of the API token"),
    ),
    security(
        ("api_token" = []),
        ("cookie" = []),
    ),
    tag = "api_tokens",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn revoke_api_token(
    app: AppState,
    Path(id): Path<i32>,
    req: Parts,
) -> AppResult<ErasedJson> {
    let mut conn = app.db_write().await?;
    let auth = AuthCheck::default().check(&req, &mut conn).await?;
    let user = auth.user();
    diesel::update(ApiToken::belonging_to(user).find(id))
        .set(api_tokens::revoked.eq(true))
        .execute(&mut conn)
        .await?;

    Ok(json!({}))
}

/// Revoke the current API token.
///
/// This endpoint revokes the API token that is used to authenticate
/// the request.
#[utoipa::path(
    delete,
    path = "/api/v1/tokens/current",
    security(("api_token" = [])),
    tag = "api_tokens",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn revoke_current_api_token(app: AppState, req: Parts) -> AppResult<Response> {
    let mut conn = app.db_write().await?;
    let auth = AuthCheck::default().check(&req, &mut conn).await?;
    let api_token_id = auth
        .api_token_id()
        .ok_or_else(|| bad_request("token not provided"))?;

    diesel::update(api_tokens::table.filter(api_tokens::id.eq(api_token_id)))
        .set(api_tokens::revoked.eq(true))
        .execute(&mut conn)
        .await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

struct NewTokenEmail<'a> {
    token_name: &'a str,
    user_name: &'a str,
    domain: &'a str,
}

impl crate::email::Email for NewTokenEmail<'_> {
    fn subject(&self) -> String {
        format!("crates.io: New API token \"{}\" created", self.token_name)
    }

    fn body(&self) -> String {
        format!(
            "\
Hello {user_name}!

A new API token with the name \"{token_name}\" was recently added to your {domain} account.

If this wasn't you, you should revoke the token immediately: https://{domain}/settings/tokens",
            token_name = self.token_name,
            user_name = self.user_name,
            domain = self.domain,
        )
    }
}
