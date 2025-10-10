use crate::email::EmailMessage;
use crate::models::ApiToken;
use crate::schema::api_tokens;
use crate::views::EncodableApiTokenWithToken;
use anyhow::Context;

use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::middleware::real_ip::RealIp;
use crate::models::token::{CrateScope, EndpointScope};
use crate::util::errors::{AppResult, bad_request, custom};
use crate::util::token::PlainToken;
use axum::Json;
use axum::extract::{Path, Query};
use axum::response::{IntoResponse, Response};
use axum_extra::json;
use axum_extra::response::ErasedJson;
use chrono::{DateTime, Utc};
use diesel::data_types::PgInterval;
use diesel::dsl::{IntervalDsl, now};
use diesel::prelude::*;
use diesel::sql_types::Timestamptz;
use diesel_async::RunQueryDsl;
use http::request::Parts;
use http::{StatusCode, header};
use minijinja::context;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

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

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ListResponse {
    pub api_tokens: Vec<ApiToken>,
}

/// List all API tokens of the authenticated user.
#[utoipa::path(
    get,
    path = "/api/v1/me/tokens",
    security(("cookie" = [])),
    tag = "api_tokens",
    responses((status = 200, description = "Successful Response", body = inline(ListResponse))),
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
                .gt(now.into_sql::<Timestamptz>() - params.expired_days_interval())),
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
    expired_at: Option<DateTime<Utc>>,
}

/// The incoming serialization format for the `ApiToken` model.
#[derive(Deserialize)]
pub struct NewApiTokenRequest {
    api_token: NewApiToken,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CreateResponse {
    api_token: EncodableApiTokenWithToken,
}

/// Create a new API token.
#[utoipa::path(
    put,
    path = "/api/v1/me/tokens",
    security(("cookie" = [])),
    tag = "api_tokens",
    responses((status = 200, description = "Successful Response", body = inline(CreateResponse))),
)]
pub async fn create_api_token(
    app: AppState,
    parts: Parts,
    Json(new): Json<NewApiTokenRequest>,
) -> AppResult<Json<CreateResponse>> {
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

    // Check if token creation is disabled
    if let Some(disable_message) = &app.config.disable_token_creation {
        let client_ip = parts.extensions.get::<RealIp>().map(|ip| ip.to_string());
        let client_ip = client_ip.as_deref().unwrap_or("unknown");

        let mut headers = parts.headers.clone();
        headers.remove(header::AUTHORIZATION);
        headers.remove(header::COOKIE);

        warn!(
            network.client.ip = client_ip,
            http.headers = ?headers,
            "Blocked token creation for user `{}` (id: {}) due to disabled flag (token name: `{}`)",
            user.gh_login, user.id, new.api_token.name
        );

        let message = disable_message.clone();
        return Err(custom(StatusCode::SERVICE_UNAVAILABLE, message));
    }

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

    let plaintext = PlainToken::generate();

    let new_token = crate::models::token::NewApiToken::builder()
        .user_id(user.id)
        .name(&new.api_token.name)
        .token(plaintext.hashed())
        .maybe_crate_scopes(crate_scopes)
        .maybe_endpoint_scopes(endpoint_scopes)
        .maybe_expired_at(new.api_token.expired_at)
        .build();

    if let Some(recipient) = recipient {
        let context = context! {
            token_name => &new.api_token.name,
            user_name => &user.gh_login,
            domain => app.emails.domain,
        };

        // At this point the token has been created so failing to send the
        // email should not cause an error response to be returned to the
        // caller.
        if let Err(e) = send_creation_email(&app.emails, &recipient, context).await {
            error!("Failed to send token creation email: {e}")
        }
    }

    let api_token = EncodableApiTokenWithToken {
        token: new_token.insert(&mut conn).await?,
        plaintext: plaintext.expose_secret().to_string(),
    };

    Ok(Json(CreateResponse { api_token }))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct GetResponse {
    pub api_token: ApiToken,
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
    responses((status = 200, description = "Successful Response", body = inline(GetResponse))),
)]
pub async fn find_api_token(
    app: AppState,
    Path(id): Path<i32>,
    req: Parts,
) -> AppResult<Json<GetResponse>> {
    let mut conn = app.db_write().await?;
    let auth = AuthCheck::default().check(&req, &mut conn).await?;
    let user = auth.user();
    let api_token = ApiToken::belonging_to(user)
        .find(id)
        .select(ApiToken::as_select())
        .first(&mut conn)
        .await?;

    Ok(Json(GetResponse { api_token }))
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
    responses((status = 200, description = "Successful Response", body = Object)),
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
    responses((status = 204, description = "Successful Response")),
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

async fn send_creation_email(
    emails: &crate::Emails,
    recipient: &str,
    context: impl Serialize,
) -> anyhow::Result<()> {
    let email = EmailMessage::from_template("new_token", context);
    let email = email.context("Failed to render email template")?;
    let result = emails.send(recipient, email).await;
    result.context("Failed to send email")
}
