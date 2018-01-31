use chrono::NaiveDateTime;
use conduit::{Request, Response};
use conduit_router::RequestParams;
use diesel::prelude::*;
use diesel;
use serde_json as json;

use db::RequestTransaction;
use user::{AuthenticationSource, RequestUser};
use util::{bad_request, read_fill, CargoResult, ChainError, RequestUtils};

use models::User;
use schema::api_tokens;

/// The model representing a row in the `api_tokens` database table.
#[derive(Clone, Debug, PartialEq, Eq, Identifiable, Queryable, Associations, Serialize)]
#[belongs_to(User)]
pub struct ApiToken {
    pub id: i32,
    #[serde(skip)] pub user_id: i32,
    #[serde(skip)] pub token: String,
    pub name: String,
    #[serde(with = "::util::rfc3339")] pub created_at: NaiveDateTime,
    #[serde(with = "::util::rfc3339::option")] pub last_used_at: Option<NaiveDateTime>,
}

/// The serialization format for the `ApiToken` model with its token value.
/// This should only be used when initially creating a new token to minimize
/// the chance of token leaks.
#[derive(Deserialize, Serialize, Debug)]
pub struct EncodableApiTokenWithToken {
    pub id: i32,
    pub name: String,
    pub token: String,
    #[serde(with = "::util::rfc3339")] pub created_at: NaiveDateTime,
    #[serde(with = "::util::rfc3339::option")] pub last_used_at: Option<NaiveDateTime>,
}

impl ApiToken {
    /// Generates a new named API token for a user
    pub fn insert(conn: &PgConnection, user_id: i32, name: &str) -> QueryResult<ApiToken> {
        diesel::insert_into(api_tokens::table)
            .values((api_tokens::user_id.eq(user_id), api_tokens::name.eq(name)))
            .get_result::<ApiToken>(conn)
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
            created_at: self.created_at,
            last_used_at: self.last_used_at,
        }
    }
}

/// Handles the `GET /me/tokens` route.
pub fn list(req: &mut Request) -> CargoResult<Response> {
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
    if name.len() < 1 {
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
pub fn revoke(req: &mut Request) -> CargoResult<Response> {
    let id = req.params()["id"]
        .parse::<i32>()
        .map_err(|e| bad_request(&format!("invalid token id: {:?}", e)))?;

    diesel::delete(ApiToken::belonging_to(req.user()?).find(id)).execute(&*req.db_conn()?)?;

    #[derive(Serialize)]
    struct R {}
    Ok(req.json(&R {}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use serde_json;

    #[test]
    fn api_token_serializes_to_rfc3339() {
        let tok = ApiToken {
            id: 12345,
            user_id: 23456,
            token: "".to_string(),
            name: "".to_string(),
            created_at: NaiveDate::from_ymd(2017, 1, 6).and_hms(14, 23, 11),
            last_used_at: Some(NaiveDate::from_ymd(2017, 1, 6).and_hms(14, 23, 12)),
        };
        let json = serde_json::to_string(&tok).unwrap();
        assert!(
            json.as_str()
                .find(r#""created_at":"2017-01-06T14:23:11+00:00""#)
                .is_some()
        );
        assert!(
            json.as_str()
                .find(r#""last_used_at":"2017-01-06T14:23:12+00:00""#)
                .is_some()
        );
    }

    #[test]
    fn encodeable_api_token_with_token_serializes_to_rfc3339() {
        let tok = EncodableApiTokenWithToken {
            id: 12345,
            name: "".to_string(),
            token: "".to_string(),
            created_at: NaiveDate::from_ymd(2017, 1, 6).and_hms(14, 23, 11),
            last_used_at: Some(NaiveDate::from_ymd(2017, 1, 6).and_hms(14, 23, 12)),
        };
        let json = serde_json::to_string(&tok).unwrap();
        assert!(
            json.as_str()
                .find(r#""created_at":"2017-01-06T14:23:11+00:00""#)
                .is_some()
        );
        assert!(
            json.as_str()
                .find(r#""last_used_at":"2017-01-06T14:23:12+00:00""#)
                .is_some()
        );
    }

}
