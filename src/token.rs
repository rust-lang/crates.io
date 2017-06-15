use std::fmt;

use diesel;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use conduit::{Request, Response};
use time::Timespec;
use conduit_router::RequestParams;
use rustc_serialize::json;

use db::RequestTransaction;
use user::{RequestUser, AuthenticationSource};
use util::{RequestUtils, CargoError, CargoResult, ChainError, human, read_fill};
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

/// The serialization format for the `ApiToken` model.
#[derive(RustcDecodable, RustcEncodable)]
pub struct EncodableApiToken {
    pub id: i32,
    pub name: String,
    pub token: Option<String>,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

impl ApiToken {
    /// Generates a new named API token for a user
    pub fn insert(conn: &PgConnection, user_id: i32, name: &str) -> CargoResult<ApiToken> {
        #[table_name = "api_tokens"]
        #[derive(Insertable, AsChangeset)]
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
    pub fn delete(conn: &PgConnection, user_id: i32, id: i32) -> CargoResult<()> {
        diesel::delete(api_tokens::table.find(id).filter(
            api_tokens::user_id.eq(user_id),
        )).execute(conn)?;
        Ok(())
    }

    pub fn find_for_user(conn: &PgConnection, user_id: i32) -> CargoResult<Vec<ApiToken>> {
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
            token: None,
            created_at: ::encode_time(self.created_at),
            last_used_at: self.last_used_at.map(::encode_time),
        }
    }

    /// Converts this `ApiToken` model into an `EncodableApiToken` including
    /// the actual token value for JSON serialization.
    pub fn encodable_with_token(self) -> EncodableApiToken {
        EncodableApiToken {
            id: self.id,
            name: self.name,
            token: Some(self.token),
            created_at: ::encode_time(self.created_at),
            last_used_at: self.last_used_at.map(::encode_time),
        }
    }
}

struct BadRequest<T: CargoError>(T);

impl<T: CargoError> CargoError for BadRequest<T> {
    fn description(&self) -> &str {
        self.0.description()
    }
    fn response(&self) -> Option<Response> {
        self.0.response().map(|mut response| {
            response.status = (400, "Bad Request");
            response
        })
    }
}

impl<T: CargoError> fmt::Display for BadRequest<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Bad Request: {}", self.0)
    }
}

fn bad_request<T: CargoError>(error: T) -> Box<CargoError> {
    Box::new(BadRequest(error))
}

/// Handles the `GET /me/tokens` route.
pub fn list(req: &mut Request) -> CargoResult<Response> {
    let tokens = ApiToken::find_for_user(&*req.db_conn()?, req.user()?.id)?
        .into_iter()
        .map(ApiToken::encodable)
        .collect();
    #[derive(RustcEncodable)]
    struct R {
        api_tokens: Vec<EncodableApiToken>,
    }
    Ok(req.json(&R { api_tokens: tokens }))
}

/// Handles the `POST /me/tokens` route.
pub fn new(req: &mut Request) -> CargoResult<Response> {
    /// The incoming serialization format for the `ApiToken` model.
    #[derive(RustcDecodable, RustcEncodable)]
    struct NewApiToken {
        pub name: String,
    }

    /// The incoming serialization format for the `ApiToken` model.
    #[derive(RustcDecodable, RustcEncodable)]
    struct NewApiTokenRequest {
        pub api_token: NewApiToken,
    }

    if req.authentication_source()? != AuthenticationSource::SessionCookie {
        return Err(bad_request(
            human("cannot use an API token to create a new API token"),
        ));
    }

    let max_post_size = 2000;
    let length = req.content_length().chain_error(|| {
        human("missing header: Content-Length")
    })?;

    if length > max_post_size {
        return Err(bad_request(
            human(&format_args!("max post size is: {}", max_post_size)),
        ));
    }

    let mut json = vec![0; length as usize];
    read_fill(req.body(), &mut json)?;

    let json = String::from_utf8(json).map_err(|_| {
        bad_request(human("json body was not valid utf-8"))
    })?;

    let new: NewApiTokenRequest = json::decode(&json).map_err(|e| {
        bad_request(human(&format_args!("invalid new token request: {:?}", e)))
    })?;

    let name = &new.api_token.name;
    if name.len() < 1 {
        return Err(bad_request(human("name must have a value")));
    }

    let user = req.user()?;

    let max_token_per_user = 500;
    let count = ApiToken::count_for_user(&*req.db_conn()?, user.id)?;
    if count >= max_token_per_user {
        return Err(bad_request(human(&format_args!(
            "maximum tokens per user is: {}",
            max_token_per_user
        ))));
    }

    let api_token = ApiToken::insert(&*req.db_conn()?, user.id, name)?;

    #[derive(RustcEncodable)]
    struct R {
        api_token: EncodableApiToken,
    }
    Ok(req.json(&R { api_token: api_token.encodable_with_token() }))
}

/// Handles the `DELETE /me/tokens/:id` route.
pub fn revoke(req: &mut Request) -> CargoResult<Response> {
    let user = req.user()?;
    let id = req.params()["id"].parse().map_err(|e| {
        bad_request(human(&format_args!("invalid token id: {:?}", e)))
    })?;

    ApiToken::delete(&*req.db_conn()?, user.id, id)?;

    #[derive(RustcEncodable)]
    struct R {}
    Ok(req.json(&R {}))
}
