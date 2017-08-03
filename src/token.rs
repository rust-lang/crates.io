use diesel;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use conduit::{Request, Response};
use time::Timespec;
use conduit_router::RequestParams;
use serde_json as json;

use db::RequestTransaction;
use user::{RequestUser, AuthenticationSource};
use util::{RequestUtils, CargoResult, ChainError, bad_request, read_fill};
use schema::api_tokens;

/// The model representing a row in the `api_tokens` database table.
#[derive(Clone, Debug, PartialEq, Eq, Identifiable, Queryable)]
pub struct ApiToken {
    pub id: i32,
    pub user_id: i32,
    pub token: String,
    pub name: String,
    pub created_at: Timespec,
    pub last_used_at: Option<Timespec>,
}

/// The serialization format for the `ApiToken` model without its token value.
#[derive(Deserialize, Serialize, Debug)]
pub struct EncodableApiToken {
    pub id: i32,
    pub name: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

/// The serialization format for the `ApiToken` model with its token value.
/// This should only be used when initially creating a new token to minimize
/// the chance of token leaks.
#[derive(Deserialize, Serialize, Debug)]
pub struct EncodableApiTokenWithToken {
    pub id: i32,
    pub name: String,
    pub token: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

impl ApiToken {
    /// Generates a new named API token for a user
    pub fn insert(conn: &PgConnection, user_id: i32, name: &str) -> QueryResult<ApiToken> {
        #[table_name = "api_tokens"]
        #[derive(Insertable, AsChangeset, Debug)]
        struct NewApiToken<'a> {
            user_id: i32,
            name: &'a str,
        }

        diesel::insert(&NewApiToken {
            user_id: user_id,
            name: name,
        }).into(api_tokens::table)
            .get_result::<ApiToken>(conn)
            .map_err(From::from)
    }

    /// Deletes the provided API token if it belongs to the provided user
    pub fn delete(conn: &PgConnection, user_id: i32, id: i32) -> QueryResult<()> {
        diesel::delete(api_tokens::table.find(id).filter(
            api_tokens::user_id.eq(user_id),
        )).execute(conn)?;
        Ok(())
    }

    pub fn find_for_user(conn: &PgConnection, user_id: i32) -> QueryResult<Vec<ApiToken>> {
        api_tokens::table
            .filter(api_tokens::user_id.eq(user_id))
            .order(api_tokens::created_at.desc())
            .load::<ApiToken>(conn)
            .map_err(From::from)
    }

    pub fn count_for_user(conn: &PgConnection, user_id: i32) -> CargoResult<u64> {
        api_tokens::table
            .filter(api_tokens::user_id.eq(user_id))
            .count()
            .get_result::<i64>(conn)
            .map(|count| count as u64)
            .map_err(From::from)
    }

    /// Converts this `ApiToken` model into an `EncodableApiToken` for JSON
    /// serialization.
    pub fn encodable(self) -> EncodableApiToken {
        EncodableApiToken {
            id: self.id,
            name: self.name,
            created_at: ::encode_time(self.created_at),
            last_used_at: self.last_used_at.map(::encode_time),
        }
    }

    /// Converts this `ApiToken` model into an `EncodableApiToken` including
    /// the actual token value for JSON serialization.  This should only be
    /// used when initially creating a new token to minimize the chance of
    /// token leaks.
    pub fn encodable_with_token(self) -> EncodableApiTokenWithToken {
        EncodableApiTokenWithToken {
            id: self.id,
            name: self.name,
            token: self.token,
            created_at: ::encode_time(self.created_at),
            last_used_at: self.last_used_at.map(::encode_time),
        }
    }
}

/// Handles the `GET /me/tokens` route.
pub fn list(req: &mut Request) -> CargoResult<Response> {
    let db_conn = &*req.db_conn()?;
    let user_id = req.user()?.id;
    let tokens = ApiToken::find_for_user(db_conn, user_id)?
        .into_iter()
        .map(ApiToken::encodable)
        .collect();
    #[derive(Serialize)]
    struct R {
        api_tokens: Vec<EncodableApiToken>,
    }
    Ok(req.json(&R { api_tokens: tokens }))
}

/// Handles the `POST /me/tokens` route.
pub fn new(req: &mut Request) -> CargoResult<Response> {
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

    let max_post_size = 2000;
    let length = req.content_length().chain_error(|| {
        bad_request("missing header: Content-Length")
    })?;

    if length > max_post_size {
        return Err(bad_request(&format!("max post size is: {}", max_post_size)));
    }

    let mut json = vec![0; length as usize];
    read_fill(req.body(), &mut json)?;

    let json = String::from_utf8(json).map_err(|_| {
        bad_request(&"json body was not valid utf-8")
    })?;

    let new: NewApiTokenRequest = json::from_str(&json).map_err(|e| {
        bad_request(&format!("invalid new token request: {:?}", e))
    })?;

    let name = &new.api_token.name;
    if name.len() < 1 {
        return Err(bad_request("name must have a value"));
    }

    let user = req.user()?;

    let max_token_per_user = 500;
    let count = ApiToken::count_for_user(&*req.db_conn()?, user.id)?;
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
    Ok(req.json(&R { api_token: api_token.encodable_with_token() }))
}

/// Handles the `DELETE /me/tokens/:id` route.
pub fn revoke(req: &mut Request) -> CargoResult<Response> {
    let user = req.user()?;
    let id = req.params()["id"].parse().map_err(|e| {
        bad_request(&format!("invalid token id: {:?}", e))
    })?;

    ApiToken::delete(&*req.db_conn()?, user.id, id)?;

    #[derive(Serialize)]
    struct R {}
    Ok(req.json(&R {}))
}
