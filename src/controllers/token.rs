use super::frontend_prelude::*;

use crate::models::ApiToken;
use crate::schema::api_tokens;
use crate::util::rfc3339;
use crate::views::EncodableApiTokenWithToken;

use crate::auth::AuthCheck;
use crate::models::token::{CrateScope, EndpointScope};
use axum::extract::Query;
use axum::response::IntoResponse;
use chrono::NaiveDateTime;
use diesel::data_types::PgInterval;
use diesel::dsl::{now, IntervalDsl};
use serde_json as json;

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
    let conn = &mut *app.db_read_prefer_primary().await?;
    conn.interact(move |conn| {
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
    .await?
}

/// Handles the `PUT /me/tokens` route.
pub async fn new(app: AppState, req: BytesRequest) -> AppResult<Json<Value>> {
    let conn = &mut *app.db_write().await?;
    conn.interact(move |conn| {
        /// The incoming serialization format for the `ApiToken` model.
        #[derive(Deserialize)]
        struct NewApiToken {
            name: String,
            crate_scopes: Option<Vec<String>>,
            endpoint_scopes: Option<Vec<String>>,
            #[serde(default, with = "rfc3339::option")]
            expired_at: Option<NaiveDateTime>,
        }

        /// The incoming serialization format for the `ApiToken` model.
        #[derive(Deserialize)]
        struct NewApiTokenRequest {
            api_token: NewApiToken,
        }

        let new: NewApiTokenRequest = json::from_slice(req.body())
            .map_err(|e| bad_request(format!("invalid new token request: {e:?}")))?;

        let name = &new.api_token.name;
        if name.is_empty() {
            return Err(bad_request("name must have a value"));
        }

        let auth = AuthCheck::default().check(&req, conn)?;
        if auth.api_token_id().is_some() {
            return Err(bad_request(
                "cannot use an API token to create a new API token",
            ));
        }

        let user = auth.user();

        let max_token_per_user = 500;
        let count: i64 = ApiToken::belonging_to(user).count().get_result(conn)?;
        if count >= max_token_per_user {
            return Err(bad_request(&format!(
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

        let api_token = ApiToken::insert_with_scopes(
            conn,
            user.id,
            name,
            crate_scopes,
            endpoint_scopes,
            new.api_token.expired_at,
        )?;
        let api_token = EncodableApiTokenWithToken::from(api_token);

        Ok(Json(json!({ "api_token": api_token })))
    })
    .await?
}

/// Handles the `DELETE /me/tokens/:id` route.
pub async fn revoke(app: AppState, Path(id): Path<i32>, req: Parts) -> AppResult<Json<Value>> {
    let conn = &mut *app.db_write().await?;
    conn.interact(move |conn| {
        let auth = AuthCheck::default().check(&req, conn)?;
        let user = auth.user();
        diesel::update(ApiToken::belonging_to(user).find(id))
            .set(api_tokens::revoked.eq(true))
            .execute(conn)?;

        Ok(Json(json!({})))
    })
    .await?
}

/// Handles the `DELETE /tokens/current` route.
pub async fn revoke_current(app: AppState, req: Parts) -> AppResult<Response> {
    let conn = &mut *app.db_write().await?;
    conn.interact(move |conn| {
        let auth = AuthCheck::default().check(&req, conn)?;
        let api_token_id = auth
            .api_token_id()
            .ok_or_else(|| bad_request("token not provided"))?;

        diesel::update(api_tokens::table.filter(api_tokens::id.eq(api_token_id)))
            .set(api_tokens::revoked.eq(true))
            .execute(conn)?;

        Ok(StatusCode::NO_CONTENT.into_response())
    })
    .await?
}
