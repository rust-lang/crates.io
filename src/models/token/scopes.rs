use crate::models::Crate;
use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::Text;
use std::io::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq, AsExpression, Serialize)]
#[diesel(sql_type = Text)]
#[serde(rename_all = "kebab-case")]
pub enum EndpointScope {
    PublishNew,
    PublishUpdate,
    Yank,
    ChangeOwners,
}

impl From<&EndpointScope> for &[u8] {
    fn from(scope: &EndpointScope) -> Self {
        match scope {
            EndpointScope::PublishNew => b"publish-new",
            EndpointScope::PublishUpdate => b"publish-update",
            EndpointScope::Yank => b"yank",
            EndpointScope::ChangeOwners => b"change-owners",
        }
    }
}

impl ToSql<Text, Pg> for EndpointScope {
    fn to_sql(&self, out: &mut Output<'_, '_, Pg>) -> serialize::Result {
        out.write_all(self.into())?;
        Ok(IsNull::No)
    }
}

impl TryFrom<&[u8]> for EndpointScope {
    type Error = String;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        match bytes {
            b"publish-new" => Ok(EndpointScope::PublishNew),
            b"publish-update" => Ok(EndpointScope::PublishUpdate),
            b"yank" => Ok(EndpointScope::Yank),
            b"change-owners" => Ok(EndpointScope::ChangeOwners),
            _ => Err("Unrecognized enum variant".to_string()),
        }
    }
}

impl FromSql<Text, Pg> for EndpointScope {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> deserialize::Result<Self> {
        let value = <String as FromSql<Text, Pg>>::from_sql(bytes)?;
        Ok(EndpointScope::try_from(value.as_bytes())?)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrateScope {
    pattern: String,
}

impl TryFrom<&str> for CrateScope {
    type Error = String;

    fn try_from(pattern: &str) -> Result<Self, Self::Error> {
        match CrateScope::is_valid_pattern(pattern) {
            true => Ok(CrateScope {
                pattern: pattern.to_string(),
            }),
            false => Err("Invalid crate scope pattern".to_string()),
        }
    }
}

impl TryFrom<String> for CrateScope {
    type Error = String;

    fn try_from(pattern: String) -> Result<Self, Self::Error> {
        match CrateScope::is_valid_pattern(&pattern) {
            true => Ok(CrateScope { pattern }),
            false => Err("Invalid crate scope pattern".to_string()),
        }
    }
}

impl FromSql<Text, Pg> for CrateScope {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> deserialize::Result<Self> {
        let value = <String as FromSql<Text, Pg>>::from_sql(bytes)?;
        Ok(CrateScope::try_from(value)?)
    }
}

impl ToSql<Text, Pg> for CrateScope {
    fn to_sql(&self, out: &mut Output<'_, '_, Pg>) -> serialize::Result {
        ToSql::<Text, Pg>::to_sql(&self.pattern, &mut out.reborrow())
    }
}

impl CrateScope {
    fn is_valid_pattern(pattern: &str) -> bool {
        if pattern.is_empty() {
            return false;
        }

        if pattern == "*" {
            return true;
        }

        let name_without_wildcard = pattern.strip_suffix('*').unwrap_or(pattern);
        Crate::valid_name(name_without_wildcard)
    }

    pub fn matches(&self, crate_name: &str) -> bool {
        if self.pattern == "*" {
            return true;
        }

        return match self.pattern.strip_suffix('*') {
            Some(prefix) => crate_name.starts_with(prefix),
            None => crate_name == self.pattern,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_scope_serialization() {
        fn assert(scope: EndpointScope, expected: &str) {
            assert_ok_eq!(serde_json::to_string(&scope), expected);
        }

        assert(EndpointScope::ChangeOwners, "\"change-owners\"");
        assert(EndpointScope::PublishNew, "\"publish-new\"");
        assert(EndpointScope::PublishUpdate, "\"publish-update\"");
        assert(EndpointScope::Yank, "\"yank\"");
    }

    #[test]
    fn crate_scope_validation() {
        assert_ok!(CrateScope::try_from("foo"));

        // wildcards
        assert_ok!(CrateScope::try_from("foo*"));
        assert_ok!(CrateScope::try_from("f*"));
        assert_ok!(CrateScope::try_from("*"));
        assert_err!(CrateScope::try_from("te*st"));

        // hyphens and underscores
        assert_ok!(CrateScope::try_from("foo-bar"));
        assert_ok!(CrateScope::try_from("foo_bar"));

        // empty string
        assert_err!(CrateScope::try_from(""));

        // invalid characters
        assert_err!(CrateScope::try_from("test#"));
    }

    #[test]
    fn crate_scope_matching() {
        let scope = |pattern: &str| CrateScope::try_from(pattern).unwrap();

        assert!(scope("foo").matches("foo"));
        assert!(!scope("foo").matches("bar"));
        assert!(!scope("foo").matches("fo"));
        assert!(!scope("foo").matches("fooo"));

        // wildcards
        assert!(scope("foo*").matches("foo"));
        assert!(!scope("foo*").matches("bar"));
        assert!(scope("foo*").matches("foo-bar"));
        assert!(scope("foo*").matches("foo_bar"));
        assert!(scope("f*").matches("foo"));
        assert!(scope("*").matches("foo"));

        // hyphens and underscores
        assert!(!scope("foo").matches("foo-bar"));
        assert!(!scope("foo").matches("foo_bar"));
        assert!(scope("foo-bar").matches("foo-bar"));
        assert!(!scope("foo-bar").matches("foo_bar"));
        assert!(!scope("foo_bar").matches("foo-bar"));
        assert!(scope("foo_bar").matches("foo_bar"));
        assert!(scope("foo-*").matches("foo-bar"));
        assert!(!scope("foo-*").matches("foo_bar"));
        assert!(!scope("foo_*").matches("foo-bar"));
        assert!(scope("foo_*").matches("foo_bar"));
    }
}
