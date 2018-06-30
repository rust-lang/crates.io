use super::prelude::*;

use diesel;
use middleware::current_user::AuthenticationSource;
use serde_json as json;
use util::{bad_request, read_fill, ChainError};

use models::ApiToken;
use schema::api_tokens;
use views::EncodableApiTokenWithToken;

/// Handles the `GET /me/tokens` route.
pub fn list(req: &mut dyn Request) -> CargoResult<Response> {
    let tokens = ApiToken::belonging_to(req.user()?)
        .order(api_tokens::created_at.desc())
        .load(&*req.db_conn()?)?;
    #[derive(Serialize)]
    struct R {
        api_tokens: Vec<ApiToken>,
    }
    Ok(req.json(&R { api_tokens: tokens }))
}

/// Handles the `PUT /me/tokens` route.
pub fn new(req: &mut dyn Request) -> CargoResult<Response> {
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

    if req.authentication_source()? != AuthenticationSource::SessionCookie {
        return Err(bad_request(
            "cannot use an API token to create a new API token",
        ));
    }

    let max_size = 2000;
    let length = req.content_length()
        .chain_error(|| bad_request("missing header: Content-Length"))?;

    if length > max_size {
        return Err(bad_request(&format!("max content length is: {}", max_size)));
    }

    let mut json = vec![0; length as usize];
    read_fill(req.body(), &mut json)?;

    let json = String::from_utf8(json).map_err(|_| bad_request(&"json body was not valid utf-8"))?;

    let new: NewApiTokenRequest = json::from_str(&json)
        .map_err(|e| bad_request(&format!("invalid new token request: {:?}", e)))?;

    let name = &new.api_token.name;
    if name.is_empty() {
        return Err(bad_request("name must have a value"));
    }

    let user = req.user()?;

    let max_token_per_user = 500;
    let count = ApiToken::belonging_to(user)
        .count()
        .get_result::<i64>(&*req.db_conn()?)?;
    if count >= max_token_per_user {
        return Err(bad_request(&format!(
            "maximum tokens per user is: {}",
            max_token_per_user
        )));
    }

    let api_token = ApiToken::insert(&*req.db_conn()?, user.id, name)?;

    #[derive(Serialize)]
    struct R {
        api_token: EncodableApiTokenWithToken,
    }
    Ok(req.json(&R {
        api_token: api_token.encodable_with_token(),
    }))
}

/// Handles the `DELETE /me/tokens/:id` route.
pub fn revoke(req: &mut dyn Request) -> CargoResult<Response> {
    let id = req.params()["id"]
        .parse::<i32>()
        .map_err(|e| bad_request(&format!("invalid token id: {:?}", e)))?;

    diesel::delete(ApiToken::belonging_to(req.user()?).find(id)).execute(&*req.db_conn()?)?;

    #[derive(Serialize)]
    struct R {}
    Ok(req.json(&R {}))
}
