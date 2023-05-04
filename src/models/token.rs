mod scopes;

use chrono::NaiveDateTime;
use diesel::prelude::*;

pub use self::scopes::{CrateScope, EndpointScope};
use crate::models::User;
use crate::schema::api_tokens;
use crate::util::errors::{AppResult, InsecurelyGeneratedTokenRevoked};
use crate::util::rfc3339;
use crate::util::token::{HashedToken, PlainToken};

/// The model representing a row in the `api_tokens` database table.
#[derive(Debug, Identifiable, Queryable, Selectable, Associations, Serialize)]
#[diesel(belongs_to(User))]
pub struct ApiToken {
    pub id: i32,
    #[serde(skip)]
    pub user_id: i32,
    pub name: String,
    #[serde(with = "rfc3339")]
    pub created_at: NaiveDateTime,
    #[serde(with = "rfc3339::option")]
    pub last_used_at: Option<NaiveDateTime>,
    #[serde(skip)]
    pub revoked: bool,
    /// `None` or a list of crate scope patterns (see RFC #2947)
    pub crate_scopes: Option<Vec<CrateScope>>,
    /// A list of endpoint scopes or `None` for the `legacy` endpoint scope (see RFC #2947)
    pub endpoint_scopes: Option<Vec<EndpointScope>>,
    #[serde(with = "rfc3339::option")]
    pub expired_at: Option<NaiveDateTime>,
}

impl ApiToken {
    /// Generates a new named API token for a user
    pub fn insert(conn: &mut PgConnection, user_id: i32, name: &str) -> AppResult<CreatedApiToken> {
        Self::insert_with_scopes(conn, user_id, name, None, None, None)
    }

    pub fn insert_with_scopes(
        conn: &mut PgConnection,
        user_id: i32,
        name: &str,
        crate_scopes: Option<Vec<CrateScope>>,
        endpoint_scopes: Option<Vec<EndpointScope>>,
        expired_at: Option<NaiveDateTime>,
    ) -> AppResult<CreatedApiToken> {
        let token = PlainToken::generate();

        let model: ApiToken = diesel::insert_into(api_tokens::table)
            .values((
                api_tokens::user_id.eq(user_id),
                api_tokens::name.eq(name),
                api_tokens::token.eq(token.hashed()),
                api_tokens::crate_scopes.eq(crate_scopes),
                api_tokens::endpoint_scopes.eq(endpoint_scopes),
                api_tokens::expired_at.eq(expired_at),
            ))
            .returning(ApiToken::as_returning())
            .get_result(conn)?;

        Ok(CreatedApiToken {
            plaintext: token,
            model,
        })
    }

    pub fn find_by_api_token(conn: &mut PgConnection, token_: &str) -> AppResult<ApiToken> {
        use crate::schema::api_tokens::dsl::*;
        use diesel::{dsl::now, update};

        let token_ =
            HashedToken::parse(token_).ok_or_else(InsecurelyGeneratedTokenRevoked::boxed)?;

        let tokens = api_tokens
            .filter(revoked.eq(false))
            .filter(expired_at.is_null().or(expired_at.gt(now)))
            .filter(token.eq(&token_));

        // If the database is in read only mode, we can't update last_used_at.
        // Try updating in a new transaction, if that fails, fall back to reading
        conn.transaction(|conn| {
            update(tokens)
                .set(last_used_at.eq(now.nullable()))
                .returning(ApiToken::as_returning())
                .get_result(conn)
        })
        .or_else(|_| tokens.select(ApiToken::as_select()).first(conn))
        .map_err(Into::into)
    }
}

#[derive(Debug)]
pub struct CreatedApiToken {
    pub model: ApiToken,
    pub plaintext: PlainToken,
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn api_token_serializes_to_rfc3339() {
        let tok = build_mock()
            .created_at(
                NaiveDate::from_ymd_opt(2017, 1, 6)
                    .unwrap()
                    .and_hms_opt(14, 23, 11)
                    .unwrap(),
            )
            .last_used_at(
                Some(
                    NaiveDate::from_ymd_opt(2017, 1, 6)
                        .unwrap()
                        .and_hms_opt(14, 23, 12),
                )
                .unwrap(),
            )
            .token();
        let json = serde_json::to_string(&tok).unwrap();
        assert_some!(json
            .as_str()
            .find(r#""created_at":"2017-01-06T14:23:11+00:00""#));
        assert_some!(json
            .as_str()
            .find(r#""last_used_at":"2017-01-06T14:23:12+00:00""#));
    }

    pub struct MockBuilder(ApiToken);

    impl MockBuilder {
        pub fn token(self) -> ApiToken {
            self.0
        }

        pub fn id(mut self, id: i32) -> Self {
            self.0.id = id;
            self
        }

        pub fn user_id(mut self, user_id: i32) -> Self {
            self.0.user_id = user_id;
            self
        }

        pub fn name(mut self, name: String) -> Self {
            self.0.name = name;
            self
        }

        pub fn created_at(mut self, created_at: NaiveDateTime) -> Self {
            self.0.created_at = created_at;
            self
        }

        pub fn last_used_at(mut self, last_used_at: Option<NaiveDateTime>) -> Self {
            self.0.last_used_at = last_used_at;
            self
        }

        pub fn revoked(mut self, revoked: bool) -> Self {
            self.0.revoked = revoked;
            self
        }

        pub fn crate_scopes(mut self, crate_scopes: Option<Vec<CrateScope>>) -> Self {
            self.0.crate_scopes = crate_scopes;
            self
        }

        pub fn endpoint_scopes(mut self, endpoint_scopes: Option<Vec<EndpointScope>>) -> Self {
            self.0.endpoint_scopes = endpoint_scopes;
            self
        }
    }

    pub fn build_mock() -> MockBuilder {
        MockBuilder(ApiToken {
            id: 12345,
            user_id: 23456,
            revoked: false,
            name: "".to_string(),
            created_at: NaiveDateTime::default(),
            last_used_at: None,
            crate_scopes: None,
            endpoint_scopes: None,
            expired_at: None,
        })
    }
}
