use super::frontend_prelude::*;

use crate::models::ApiToken;
use crate::schema::api_tokens;
use crate::views::EncodableApiTokenWithToken;

use crate::auth::AuthCheck;
use axum::response::IntoResponse;
use serde_json as json;

/// Handles the `GET /me/tokens` route.
pub async fn list(req: ConduitRequest) -> AppResult<Json<Value>> {
    conduit_compat(move || {
        let conn = req.app().db_read_prefer_primary()?;
        let auth = AuthCheck::only_cookie().check(&req, &conn)?;
        let user = auth.user();

        let tokens: Vec<ApiToken> = ApiToken::belonging_to(user)
            .filter(api_tokens::revoked.eq(false))
            .order(api_tokens::created_at.desc())
            .load(&*conn)?;

        Ok(Json(json!({ "api_tokens": tokens })))
    })
    .await
}

/// Handles the `PUT /me/tokens` route.
pub async fn new(req: ConduitRequest) -> AppResult<Json<Value>> {
    conduit_compat(move || {
        /// The incoming serialization format for the `ApiToken` model.
        #[derive(Deserialize, Serialize)]
        struct NewApiToken {
            name: String,
        }

        /// The incoming serialization format for the `ApiToken` model.
        #[derive(Deserialize, Serialize)]
        struct NewApiTokenRequest {
            api_token: NewApiToken,
        }

        let new: NewApiTokenRequest = json::from_slice(req.body())
            .map_err(|e| bad_request(&format!("invalid new token request: {e:?}")))?;

        let name = &new.api_token.name;
        if name.is_empty() {
            return Err(bad_request("name must have a value"));
        }

        let conn = req.app().db_write()?;

        let auth = AuthCheck::default().check(&req, &conn)?;
        if auth.api_token_id().is_some() {
            return Err(bad_request(
                "cannot use an API token to create a new API token",
            ));
        }

        let user = auth.user();

        let max_token_per_user = 500;
        let count: i64 = ApiToken::belonging_to(user).count().get_result(&*conn)?;
        if count >= max_token_per_user {
            return Err(bad_request(&format!(
                "maximum tokens per user is: {max_token_per_user}"
            )));
        }

        let api_token = ApiToken::insert(&conn, user.id, name)?;
        let api_token = EncodableApiTokenWithToken::from(api_token);

        Ok(Json(json!({ "api_token": api_token })))
    })
    .await
}

/// Handles the `DELETE /me/tokens/:id` route.
pub async fn revoke(Path(id): Path<i32>, req: ConduitRequest) -> AppResult<Json<Value>> {
    conduit_compat(move || {
        let conn = req.app().db_write()?;
        let auth = AuthCheck::default().check(&req, &conn)?;
        let user = auth.user();
        diesel::update(ApiToken::belonging_to(user).find(id))
            .set(api_tokens::revoked.eq(true))
            .execute(&*conn)?;

        Ok(Json(json!({})))
    })
    .await
}

/// Handles the `DELETE /tokens/current` route.
pub async fn revoke_current(req: ConduitRequest) -> AppResult<Response> {
    conduit_compat(move || {
        let conn = req.app().db_write()?;
        let auth = AuthCheck::default().check(&req, &conn)?;
        let api_token_id = auth
            .api_token_id()
            .ok_or_else(|| bad_request("token not provided"))?;

        diesel::update(api_tokens::table.filter(api_tokens::id.eq(api_token_id)))
            .set(api_tokens::revoked.eq(true))
            .execute(&*conn)?;

        Ok(StatusCode::NO_CONTENT.into_response())
    })
    .await
}
