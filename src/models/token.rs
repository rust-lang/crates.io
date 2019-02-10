use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::models::User;
use crate::schema::api_tokens;
use crate::util::rfc3339;
use crate::views::EncodableApiTokenWithToken;

/// The model representing a row in the `api_tokens` database table.
#[derive(Clone, Debug, PartialEq, Eq, Identifiable, Queryable, Associations, Serialize)]
#[belongs_to(User)]
pub struct ApiToken {
    pub id: i32,
    #[serde(skip)]
    pub user_id: i32,
    #[serde(skip)]
    pub token: String,
    pub name: String,
    #[serde(with = "rfc3339")]
    pub created_at: NaiveDateTime,
    #[serde(with = "rfc3339::option")]
    pub last_used_at: Option<NaiveDateTime>,
    #[serde(skip)]
    pub revoked: bool,
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
            revoked: self.revoked,
            created_at: self.created_at,
            last_used_at: self.last_used_at,
        }
    }
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
            revoked: false,
            name: "".to_string(),
            created_at: NaiveDate::from_ymd(2017, 1, 6).and_hms(14, 23, 11),
            last_used_at: Some(NaiveDate::from_ymd(2017, 1, 6).and_hms(14, 23, 12)),
        };
        let json = serde_json::to_string(&tok).unwrap();
        assert!(json
            .as_str()
            .find(r#""created_at":"2017-01-06T14:23:11+00:00""#)
            .is_some());
        assert!(json
            .as_str()
            .find(r#""last_used_at":"2017-01-06T14:23:12+00:00""#)
            .is_some());
    }

    #[test]
    fn encodeable_api_token_with_token_serializes_to_rfc3339() {
        let tok = EncodableApiTokenWithToken {
            id: 12345,
            name: "".to_string(),
            token: "".to_string(),
            revoked: false,
            created_at: NaiveDate::from_ymd(2017, 1, 6).and_hms(14, 23, 11),
            last_used_at: Some(NaiveDate::from_ymd(2017, 1, 6).and_hms(14, 23, 12)),
        };
        let json = serde_json::to_string(&tok).unwrap();
        assert!(json
            .as_str()
            .find(r#""created_at":"2017-01-06T14:23:11+00:00""#)
            .is_some());
        assert!(json
            .as_str()
            .find(r#""last_used_at":"2017-01-06T14:23:12+00:00""#)
            .is_some());
    }

}
