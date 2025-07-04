use diesel::deserialize::{self, FromSql};
use diesel::pg::{Pg, PgValue};
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Jsonb;
use diesel::{AsExpression, FromSqlRow};
use serde::{Deserialize, Serialize};

/// Data structure containing trusted publisher information extracted from JWT claims
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Jsonb)]
#[serde(tag = "provider")]
pub enum TrustpubData {
    #[serde(rename = "github")]
    GitHub {
        /// Repository (e.g. "octo-org/octo-repo")
        repository: String,
        /// Workflow run ID
        run_id: String,
        /// SHA of the commit
        sha: String,
    },
}

impl ToSql<Jsonb, Pg> for TrustpubData {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let json = serde_json::to_value(self)?;
        <serde_json::Value as ToSql<Jsonb, Pg>>::to_sql(&json, &mut out.reborrow())
    }
}

impl FromSql<Jsonb, Pg> for TrustpubData {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let json = <serde_json::Value as FromSql<Jsonb, Pg>>::from_sql(bytes)?;
        Ok(serde_json::from_value(json)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_json_snapshot;

    #[test]
    fn test_serialization() {
        let data = TrustpubData::GitHub {
            repository: "octo-org/octo-repo".to_string(),
            run_id: "example-run-id".to_string(),
            sha: "example-sha".to_string(),
        };

        assert_json_snapshot!(data, @r#"
        {
          "provider": "github",
          "repository": "octo-org/octo-repo",
          "run_id": "example-run-id",
          "sha": "example-sha"
        }
        "#);
    }
}
