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

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
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
        Crate::validate_crate_name("crate", name_without_wildcard).is_ok()
    }

    pub fn matches(&self, crate_name: &str) -> bool {
        if self.pattern == "*" {
            return true;
        }

        match self.pattern.strip_suffix('*') {
            Some(prefix) => crate_name.starts_with(prefix),
            None => crate_name == self.pattern,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;

    #[googletest::test]
    fn endpoint_scope_serialization() {
        fn assert(scope: EndpointScope, expected: &str) {
            expect_that!(serde_json::to_string(&scope), ok(eq(expected)));
        }

        assert(EndpointScope::ChangeOwners, "\"change-owners\"");
        assert(EndpointScope::PublishNew, "\"publish-new\"");
        assert(EndpointScope::PublishUpdate, "\"publish-update\"");
        assert(EndpointScope::Yank, "\"yank\"");
    }

    #[googletest::test]
    fn crate_scope_serialization() {
        fn assert(scope: &str, expected: &str) {
            let scope = assert_ok!(CrateScope::try_from(scope));
            expect_that!(serde_json::to_string(&scope), ok(eq(expected)));
        }

        assert("foo", "\"foo\"");
        assert("foo*", "\"foo*\"");
        assert("f*", "\"f*\"");
        assert("*", "\"*\"");
        assert("foo-bar", "\"foo-bar\"");
        assert("foo_bar", "\"foo_bar\"");
        assert("FooBar", "\"FooBar\"");
    }

    #[googletest::test]
    fn crate_scope_validation() {
        expect_that!(CrateScope::try_from("foo"), ok(anything()));

        // wildcards
        expect_that!(CrateScope::try_from("foo*"), ok(anything()));
        expect_that!(CrateScope::try_from("f*"), ok(anything()));
        expect_that!(CrateScope::try_from("*"), ok(anything()));
        expect_that!(CrateScope::try_from("te*st"), err(anything()));

        // hyphens and underscores
        expect_that!(CrateScope::try_from("foo-bar"), ok(anything()));
        expect_that!(CrateScope::try_from("foo_bar"), ok(anything()));

        // empty string
        expect_that!(CrateScope::try_from(""), err(anything()));

        // invalid characters
        expect_that!(CrateScope::try_from("test#"), err(anything()));
    }

    #[googletest::test]
    fn crate_scope_matching() {
        let scope = |pattern: &str| CrateScope::try_from(pattern).unwrap();

        expect_that!(scope("foo").matches("foo"), eq(true));
        expect_that!(scope("foo").matches("bar"), eq(false));
        expect_that!(scope("foo").matches("fo"), eq(false));
        expect_that!(scope("foo").matches("fooo"), eq(false));

        // wildcards
        expect_that!(scope("foo*").matches("foo"), eq(true));
        expect_that!(scope("foo*").matches("bar"), eq(false));
        expect_that!(scope("foo*").matches("foo-bar"), eq(true));
        expect_that!(scope("foo*").matches("foo_bar"), eq(true));
        expect_that!(scope("f*").matches("foo"), eq(true));
        expect_that!(scope("*").matches("foo"), eq(true));

        // hyphens and underscores
        expect_that!(scope("foo").matches("foo-bar"), eq(false));
        expect_that!(scope("foo").matches("foo_bar"), eq(false));
        expect_that!(scope("foo-bar").matches("foo-bar"), eq(true));
        expect_that!(scope("foo-bar").matches("foo_bar"), eq(false));
        expect_that!(scope("foo_bar").matches("foo-bar"), eq(false));
        expect_that!(scope("foo_bar").matches("foo_bar"), eq(true));
        expect_that!(scope("foo-*").matches("foo-bar"), eq(true));
        expect_that!(scope("foo-*").matches("foo_bar"), eq(false));
        expect_that!(scope("foo_*").matches("foo-bar"), eq(false));
        expect_that!(scope("foo_*").matches("foo_bar"), eq(true));
    }
}
