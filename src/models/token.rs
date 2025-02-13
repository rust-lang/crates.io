mod scopes;

use bon::Builder;
use chrono::NaiveDateTime;
use diesel::dsl::now;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};

pub use self::scopes::{CrateScope, EndpointScope};
use crate::models::User;
use crate::schema::api_tokens;
use crate::util::rfc3339;
use crate::util::token::{HashedToken, PlainToken};

#[derive(Debug, Insertable, Builder)]
#[diesel(table_name = api_tokens, check_for_backend(diesel::pg::Pg))]
pub struct NewApiToken {
    pub user_id: i32,
    #[builder(into)]
    pub name: String,
    #[builder(default = PlainToken::generate().hashed())]
    pub token: HashedToken,
    /// `None` or a list of crate scope patterns (see RFC #2947)
    pub crate_scopes: Option<Vec<CrateScope>>,
    /// A list of endpoint scopes or `None` for the `legacy` endpoint scope (see RFC #2947)
    pub endpoint_scopes: Option<Vec<EndpointScope>>,
    pub expired_at: Option<NaiveDateTime>,
}

impl NewApiToken {
    pub async fn insert(&self, conn: &mut AsyncPgConnection) -> QueryResult<ApiToken> {
        diesel::insert_into(api_tokens::table)
            .values(self)
            .returning(ApiToken::as_returning())
            .get_result(conn)
            .await
    }
}

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
    pub async fn find_by_api_token(
        conn: &mut AsyncPgConnection,
        token: &HashedToken,
    ) -> QueryResult<ApiToken> {
        let tokens = api_tokens::table
            .filter(api_tokens::revoked.eq(false))
            .filter(
                api_tokens::expired_at
                    .is_null()
                    .or(api_tokens::expired_at.gt(now)),
            )
            .filter(api_tokens::token.eq(token));

        // If the database is in read only mode, we can't update last_used_at.
        // Try updating in a new transaction, if that fails, fall back to reading
        let token = conn
            .transaction(|conn| {
                async move {
                    diesel::update(tokens)
                        .set(api_tokens::last_used_at.eq(now.nullable()))
                        .returning(ApiToken::as_returning())
                        .get_result(conn)
                        .await
                }
                .scope_boxed()
            })
            .await;
        let Ok(_) = token else {
            return tokens.select(ApiToken::as_select()).first(conn).await;
        };
        token
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn api_token_serializes_to_rfc3339() {
        let tok = ApiToken {
            id: 12345,
            user_id: 23456,
            revoked: false,
            name: "".to_string(),
            created_at: NaiveDate::from_ymd_opt(2017, 1, 6)
                .unwrap()
                .and_hms_opt(14, 23, 11)
                .unwrap(),
            last_used_at: NaiveDate::from_ymd_opt(2017, 1, 6)
                .unwrap()
                .and_hms_opt(14, 23, 12),
            crate_scopes: None,
            endpoint_scopes: None,
            expired_at: None,
        };
        let json = serde_json::to_string(&tok).unwrap();
        assert_some!(json
            .as_str()
            .find(r#""created_at":"2017-01-06T14:23:11+00:00""#));
        assert_some!(json
            .as_str()
            .find(r#""last_used_at":"2017-01-06T14:23:12+00:00""#));
    }
}
