use crate::models::ApiToken;
use crate::schema::api_tokens;
use crate::util::rfc3339;
use crate::views::EncodableApiTokenWithToken;

use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::models::token::{CrateScope, EndpointScope};
use crate::tasks::spawn_blocking;
use crate::util::errors::{bad_request, AppResult};
use axum::extract::{Path, Query};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::NaiveDateTime;
use diesel::data_types::PgInterval;
use diesel::dsl::{now, IntervalDsl};
use diesel::prelude::*;
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use http::request::Parts;
use http::StatusCode;
use serde_json::Value;

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

/// Handles the `GET /me/tokens` route.
pub async fn list(
    app: AppState,
    Query(params): Query<GetParams>,
    req: Parts,
) -> AppResult<Json<Value>> {
    let conn = app.db_read_prefer_primary().await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        let auth = AuthCheck::only_cookie().check(&req, conn)?;
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
            .load(conn)?;

        Ok(Json(json!({ "api_tokens": tokens })))
    })
    .await
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

/// Handles the `PUT /me/tokens` route.
pub async fn new(
    app: AppState,
    parts: Parts,
    Json(new): Json<NewApiTokenRequest>,
) -> AppResult<Json<Value>> {
    if new.api_token.name.is_empty() {
        return Err(bad_request("name must have a value"));
    }

    let conn = app.db_write().await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        let auth = AuthCheck::default().check(&parts, conn)?;
        if auth.api_token_id().is_some() {
            return Err(bad_request(
                "cannot use an API token to create a new API token",
            ));
        }

        let user = auth.user();

        let max_token_per_user = 500;
        let count: i64 = ApiToken::belonging_to(user).count().get_result(conn)?;
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

        let recipient = user.email(conn)?;

        let api_token = ApiToken::insert_with_scopes(
            conn,
            user.id,
            &new.api_token.name,
            crate_scopes,
            endpoint_scopes,
            new.api_token.expired_at,
        )?;

        if let Some(recipient) = recipient {
            let email = NewTokenEmail {
                token_name: &new.api_token.name,
                user_name: &user.gh_login,
                domain: &app.emails.domain,
            };

            // At this point the token has been created so failing to send the
            // email should not cause an error response to be returned to the
            // caller.
            let email_ret = app.emails.send(&recipient, email);
            if let Err(e) = email_ret {
                error!("Failed to send token creation email: {e}")
            }
        }

        let api_token = EncodableApiTokenWithToken::from(api_token);

        Ok(Json(json!({ "api_token": api_token })))
    })
    .await
}

/// Handles the `GET /me/tokens/:id` route.
pub async fn show(app: AppState, Path(id): Path<i32>, req: Parts) -> AppResult<Json<Value>> {
    let conn = app.db_write().await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        let auth = AuthCheck::default().check(&req, conn)?;
        let user = auth.user();
        let token = ApiToken::belonging_to(user)
            .find(id)
            .select(ApiToken::as_select())
            .first(conn)?;

        Ok(Json(json!({ "api_token": token })))
    })
    .await
}

/// Handles the `DELETE /me/tokens/:id` route.
pub async fn revoke(app: AppState, Path(id): Path<i32>, req: Parts) -> AppResult<Json<Value>> {
    let conn = app.db_write().await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        let auth = AuthCheck::default().check(&req, conn)?;
        let user = auth.user();
        diesel::update(ApiToken::belonging_to(user).find(id))
            .set(api_tokens::revoked.eq(true))
            .execute(conn)?;

        Ok(Json(json!({})))
    })
    .await
}

/// Handles the `DELETE /tokens/current` route.
pub async fn revoke_current(app: AppState, req: Parts) -> AppResult<Response> {
    let conn = app.db_write().await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        let auth = AuthCheck::default().check(&req, conn)?;
        let api_token_id = auth
            .api_token_id()
            .ok_or_else(|| bad_request("token not provided"))?;

        diesel::update(api_tokens::table.filter(api_tokens::id.eq(api_token_id)))
            .set(api_tokens::revoked.eq(true))
            .execute(conn)?;

        Ok(StatusCode::NO_CONTENT.into_response())
    })
    .await
}

struct NewTokenEmail<'a> {
    token_name: &'a str,
    user_name: &'a str,
    domain: &'a str,
}

impl<'a> crate::email::Email for NewTokenEmail<'a> {
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
