use std::io::Write;
use std::str::FromStr;

use diesel::deserialize::{self, FromSql};
use diesel::pg::{Pg, PgValue};
use diesel::query_builder::QueryId;
use diesel::serialize::{self, IsNull, Output, ToSql};

use crate::schema::sql_types::OauthProvider as OauthProviderSql;

// Diesel's `#[derive(SqlType)]` does not emit `QueryId`. Binding an
// `OAuthProviderId` value into a query path requires it, so we implement it
// here rather than patching generated schema.rs.
impl QueryId for OauthProviderSql {
    type QueryId = OauthProviderSql;
    const HAS_STATIC_QUERY_ID: bool = true;
}

/// Identifier for an OAuth provider that a `User` can be associated with.
///
/// Maps to the `oauth_provider` Postgres enum type. The `OAuthProvider`
/// trait in the main crate represents provider *behavior*; this enum
/// represents provider *identity* (which provider a row refers to).
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    diesel::FromSqlRow,
    diesel::AsExpression,
)]
#[diesel(sql_type = OauthProviderSql)]
#[serde(rename_all = "snake_case")]
pub enum OAuthProviderId {
    Github,
}

impl OAuthProviderId {
    pub fn as_str(&self) -> &'static str {
        match self {
            OAuthProviderId::Github => "github",
        }
    }
}

impl FromStr for OAuthProviderId {
    type Err = UnknownOAuthProvider;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "github" => Ok(OAuthProviderId::Github),
            other => Err(UnknownOAuthProvider(other.to_string())),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown oauth provider: {0}")]
pub struct UnknownOAuthProvider(pub String);

impl FromSql<OauthProviderSql, Pg> for OAuthProviderId {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let s = std::str::from_utf8(bytes.as_bytes())?;
        Ok(s.parse()?)
    }
}

impl ToSql<OauthProviderSql, Pg> for OAuthProviderId {
    fn to_sql(&self, out: &mut Output<'_, '_, Pg>) -> serialize::Result {
        out.write_all(self.as_str().as_bytes())?;
        Ok(IsNull::No)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_roundtrips_through_from_str() {
        let s = OAuthProviderId::Github.as_str();
        let parsed: OAuthProviderId = s.parse().expect("as_str output must parse back");
        assert_eq!(parsed, OAuthProviderId::Github);
    }

    #[test]
    fn from_str_rejects_unknown_provider() {
        let err = "gitlab"
            .parse::<OAuthProviderId>()
            .expect_err("unknown provider must fail");
        assert_eq!(err.0, "gitlab");
    }

    #[test]
    fn serde_serializes_to_snake_case() {
        let s = serde_json::to_string(&OAuthProviderId::Github).unwrap();
        assert_eq!(s, "\"github\"");
    }
}
