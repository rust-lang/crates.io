use super::frontend_prelude::*;

use crate::models::ApiToken;
use crate::schema::api_tokens;
use crate::util::read_fill;
use crate::views::EncodableApiTokenWithToken;

use conduit::{Body, Response};
use serde_json as json;

/// Handles the `GET /me/tokens` route.
pub fn list(req: &mut dyn RequestExt) -> EndpointResult {
    let authenticated_user = req.authenticate()?.forbid_api_token_auth()?;
    let conn = req.db_conn()?;
    let user = authenticated_user.user();

    let tokens: Vec<ApiToken> = ApiToken::belonging_to(&user)
        .filter(api_tokens::revoked.eq(false))
        .order(api_tokens::created_at.desc())
        .load(&*conn)?;

    Ok(req.json(&json!({ "api_tokens": tokens })))
}

/// Handles the `PUT /me/tokens` route.
pub fn new(req: &mut dyn RequestExt) -> EndpointResult {
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

    let max_size = 2000;
    let length = req
        .content_length()
        .ok_or_else(|| bad_request("missing header: Content-Length"))?;

    if length > max_size {
        return Err(bad_request(&format!("max content length is: {max_size}")));
    }

    let mut json = vec![0; length as usize];
    read_fill(req.body(), &mut json)?;

    let json =
        String::from_utf8(json).map_err(|_| bad_request(&"json body was not valid utf-8"))?;

    let new: NewApiTokenRequest = json::from_str(&json)
        .map_err(|e| bad_request(&format!("invalid new token request: {e:?}")))?;

    let name = &new.api_token.name;
    if name.is_empty() {
        return Err(bad_request("name must have a value"));
    }

    let authenticated_user = req.authenticate()?;
    if authenticated_user.api_token_id().is_some() {
        return Err(bad_request(
            "cannot use an API token to create a new API token",
        ));
    }

    let conn = req.db_conn()?;
    let user = authenticated_user.user();

    let max_token_per_user = 500;
    let count: i64 = ApiToken::belonging_to(&user).count().get_result(&*conn)?;
    if count >= max_token_per_user {
        return Err(bad_request(&format!(
            "maximum tokens per user is: {}",
            max_token_per_user
        )));
    }

    let api_token = ApiToken::insert(&*conn, user.id, name)?;
    let api_token = EncodableApiTokenWithToken::from(api_token);

    Ok(req.json(&json!({ "api_token": api_token })))
}

/// Handles the `DELETE /me/tokens/:id` route.
pub fn revoke(req: &mut dyn RequestExt) -> EndpointResult {
    let id = req.params()["id"]
        .parse::<i32>()
        .map_err(|e| bad_request(&format!("invalid token id: {e:?}")))?;

    let authenticated_user = req.authenticate()?;
    let conn = req.db_conn()?;
    let user = authenticated_user.user();
    diesel::update(ApiToken::belonging_to(&user).find(id))
        .set(api_tokens::revoked.eq(true))
        .execute(&*conn)?;

    Ok(req.json(&json!({})))
}

/// Handles the `DELETE /tokens/current` route.
pub fn revoke_current(req: &mut dyn RequestExt) -> EndpointResult {
    let authenticated_user = req.authenticate()?;
    let api_token_id = authenticated_user
        .api_token_id()
        .ok_or_else(|| bad_request("token not provided"))?;

    let conn = req.db_conn()?;
    diesel::update(api_tokens::table.filter(api_tokens::id.eq(api_token_id)))
        .set(api_tokens::revoked.eq(true))
        .execute(&*conn)?;

    Ok(Response::builder().status(204).body(Body::empty()).unwrap())
}
