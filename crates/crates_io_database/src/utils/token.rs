use diesel::deserialize::FromSql;
use diesel::pg::Pg;
use diesel::serialize::ToSql;
use diesel::sql_types::Bytea;
use diesel::{AsExpression, FromSqlRow};
use rand::distr::{Alphanumeric, SampleString};
use secrecy::{ExposeSecret, SecretSlice, SecretString};
use sha2::{Digest, Sha256};

const TOKEN_LENGTH: usize = 32;

/// NEVER CHANGE THE PREFIX OF EXISTING TOKENS!!! Doing so will implicitly
/// revoke all the tokens, disrupting production users.
const TOKEN_PREFIX: &str = "cio";

/// An error indicating that a token is invalid.
///
/// This error is returned when a token is not prefixed with a
/// known crates.io-specific prefix.
#[derive(Debug, thiserror::Error)]
#[error("invalid token format")]
pub struct InvalidTokenError;

#[derive(FromSqlRow, AsExpression)]
#[diesel(sql_type = Bytea)]
pub struct HashedToken(SecretSlice<u8>);

impl HashedToken {
    pub fn parse(plaintext: &str) -> Result<Self, InvalidTokenError> {
        // This will both reject tokens without a prefix and tokens of the wrong kind.
        if !plaintext.starts_with(TOKEN_PREFIX) {
            return Err(InvalidTokenError);
        }

        let sha256 = Self::hash(plaintext).into();
        Ok(Self(sha256))
    }

    pub fn hash(plaintext: &str) -> Vec<u8> {
        Sha256::digest(plaintext.as_bytes()).as_slice().to_vec()
    }
}

impl std::fmt::Debug for HashedToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HashedToken")
    }
}

impl ToSql<Bytea, Pg> for HashedToken {
    fn to_sql(&self, out: &mut diesel::serialize::Output<'_, '_, Pg>) -> diesel::serialize::Result {
        ToSql::<Bytea, Pg>::to_sql(&self.0.expose_secret(), &mut out.reborrow())
    }
}

impl FromSql<Bytea, Pg> for HashedToken {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self> {
        let bytes: Vec<u8> = FromSql::<Bytea, Pg>::from_sql(bytes)?;
        Ok(Self(bytes.into()))
    }
}

#[derive(Debug)]
pub struct PlainToken(SecretString);

impl PlainToken {
    pub fn generate() -> Self {
        let plaintext = format!(
            "{}{}",
            TOKEN_PREFIX,
            generate_secure_alphanumeric_string(TOKEN_LENGTH)
        )
        .into();

        Self(plaintext)
    }

    pub fn hashed(&self) -> HashedToken {
        let sha256 = HashedToken::hash(self.expose_secret()).into();
        HashedToken(sha256)
    }
}

impl ExposeSecret<str> for PlainToken {
    fn expose_secret(&self) -> &str {
        self.0.expose_secret()
    }
}

fn generate_secure_alphanumeric_string(len: usize) -> String {
    Alphanumeric.sample_string(&mut rand::rng(), len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_err;
    use googletest::prelude::*;

    #[test]
    fn test_generated_and_parse() {
        let token = PlainToken::generate();
        assert_that!(token.expose_secret(), starts_with(TOKEN_PREFIX));
        assert_eq!(
            token.hashed().0.expose_secret(),
            Sha256::digest(token.expose_secret().as_bytes()).as_slice()
        );

        let parsed =
            HashedToken::parse(token.expose_secret()).expect("failed to parse back the token");
        assert_eq!(parsed.0.expose_secret(), token.hashed().0.expose_secret());
    }

    #[test]
    fn test_parse_no_kind() {
        assert_err!(HashedToken::parse("nokind"));
    }
}
