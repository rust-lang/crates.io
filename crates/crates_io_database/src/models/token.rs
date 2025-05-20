mod scopes;

use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::dsl::now;
use diesel::prelude::*;
use diesel::sql_types::Timestamptz;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};

pub use self::scopes::{CrateScope, EndpointScope};
use crate::models::User;
use crate::schema::api_tokens;
use crate::utils::token::{HashedToken, PlainToken};

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
    pub expired_at: Option<DateTime<Utc>>,
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
#[derive(
    Debug, Identifiable, Queryable, Selectable, Associations, serde::Serialize, utoipa::ToSchema,
)]
#[diesel(belongs_to(User))]
pub struct ApiToken {
    /// An opaque unique identifier for the token.
    #[schema(example = 42)]
    pub id: i32,

    #[serde(skip)]
    pub user_id: i32,

    /// The name of the token.
    #[schema(example = "Example API Token")]
    pub name: String,

    /// The date and time when the token was created.
    #[schema(example = "2017-01-06T14:23:11Z")]
    pub created_at: DateTime<Utc>,

    /// The date and time when the token was last used.
    #[schema(example = "2021-10-26T11:32:12Z")]
    pub last_used_at: Option<DateTime<Utc>>,

    #[serde(skip)]
    pub revoked: bool,

    /// `None` or a list of crate scope patterns (see RFC #2947).
    #[schema(value_type = Option<Vec<String>>, example = json!(["serde"]))]
    pub crate_scopes: Option<Vec<CrateScope>>,

    /// A list of endpoint scopes or `None` for the `legacy` endpoint scope (see RFC #2947).
    #[schema(example = json!(["publish-update"]))]
    pub endpoint_scopes: Option<Vec<EndpointScope>>,

    /// The date and time when the token will expire, or `null`.
    #[schema(example = "2030-10-26T11:32:12Z")]
    pub expired_at: Option<DateTime<Utc>>,
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
                        .set(api_tokens::last_used_at.eq(now.into_sql::<Timestamptz>().nullable()))
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
    use claims::assert_some;

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
                .unwrap()
                .and_utc(),
            last_used_at: Some(
                NaiveDate::from_ymd_opt(2017, 1, 6)
                    .unwrap()
                    .and_hms_opt(14, 23, 12)
                    .unwrap()
                    .and_utc(),
            ),
            crate_scopes: None,
            endpoint_scopes: None,
            expired_at: None,
        };
        let json = serde_json::to_string(&tok).unwrap();
        assert_some!(json.as_str().find(r#""created_at":"2017-01-06T14:23:11Z""#));
        assert_some!(
            json.as_str()
                .find(r#""last_used_at":"2017-01-06T14:23:12Z""#)
        );
    }
}
