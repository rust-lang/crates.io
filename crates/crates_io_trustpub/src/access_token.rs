use rand::distr::{Alphanumeric, SampleString};
use secrecy::{ExposeSecret, SecretString};
use sha2::digest::Output;
use sha2::{Digest, Sha256};
use std::str::FromStr;

/// A temporary access token used to publish crates to crates.io using
/// the "Trusted Publishing" feature.
///
/// The token consists of a prefix, a random alphanumeric string (31 characters),
/// and a single-character checksum.
#[derive(Debug)]
pub struct AccessToken(SecretString);

impl AccessToken {
    /// The prefix used for the temporary access token.
    ///
    /// This overlaps with the `cio` prefix used for other tokens, but since
    /// the regular tokens don't use `_` characters, they can easily be
    /// distinguished.
    pub const PREFIX: &str = "cio_tp_";

    /// The length of the random alphanumeric string in the token, without
    /// the checksum.
    const RAW_LENGTH: usize = 31;

    /// Generate a new random access token.
    pub fn generate() -> Self {
        let raw = Alphanumeric.sample_string(&mut rand::rng(), Self::RAW_LENGTH);
        Self(raw.into())
    }

    /// Wrap the raw access token with the token prefix and a checksum.
    ///
    /// This turns e.g. `ABC` into `cio_tp_ABC{checksum}`.
    pub fn finalize(&self) -> SecretString {
        let raw = self.0.expose_secret();
        let checksum = checksum(raw.as_bytes());
        format!("{}{raw}{checksum}", Self::PREFIX).into()
    }

    /// Generate a SHA256 hash of the access token.
    ///
    /// This is used to create a hashed version of the token for storage in
    /// the database to avoid storing the plaintext token.
    pub fn sha256(&self) -> Output<Sha256> {
        Sha256::digest(self.0.expose_secret())
    }
}

impl FromStr for AccessToken {
    type Err = AccessTokenError;

    /// Parse a string into an access token.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let suffix = s
            .strip_prefix(Self::PREFIX)
            .ok_or(AccessTokenError::MissingPrefix)?;

        if suffix.len() != Self::RAW_LENGTH + 1 {
            return Err(AccessTokenError::InvalidLength);
        }

        if !suffix.chars().all(|c| char::is_ascii_alphanumeric(&c)) {
            return Err(AccessTokenError::InvalidCharacter);
        }

        let raw = suffix.chars().take(Self::RAW_LENGTH).collect::<String>();
        let claimed_checksum = suffix.chars().nth(Self::RAW_LENGTH).unwrap();
        let actual_checksum = checksum(raw.as_bytes());
        if claimed_checksum != actual_checksum {
            return Err(AccessTokenError::InvalidChecksum {
                claimed: claimed_checksum,
                actual: actual_checksum,
            });
        }

        Ok(Self(raw.into()))
    }
}

/// The error type for parsing access tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessTokenError {
    MissingPrefix,
    InvalidLength,
    InvalidCharacter,
    InvalidChecksum { claimed: char, actual: char },
}

/// Generate a single-character checksum for the given raw token.
///
/// Note that this checksum is not cryptographically secure and should not be
/// used for security purposes. It should only be used to detect invalid tokens.
fn checksum(raw: &[u8]) -> char {
    const ALPHANUMERIC: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

    let checksum = raw.iter().fold(0, |acc, &b| acc ^ b);

    ALPHANUMERIC
        .chars()
        .nth(checksum as usize % ALPHANUMERIC.len())
        .unwrap_or('0')
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err_eq, assert_ok};
    use insta::{assert_compact_debug_snapshot, assert_snapshot};

    const EXAMPLE_TOKEN: &str = "gGK6jurSwKyl9V3Az19z7YEFQI9aoOO";

    #[test]
    fn test_generate() {
        let token = AccessToken::generate();
        assert_eq!(token.0.expose_secret().len(), AccessToken::RAW_LENGTH);
    }

    #[test]
    fn test_finalize() {
        let token = AccessToken(SecretString::from(EXAMPLE_TOKEN));
        assert_snapshot!(token.finalize().expose_secret(), @"cio_tp_gGK6jurSwKyl9V3Az19z7YEFQI9aoOOd");
    }

    #[test]
    fn test_sha256() {
        let token = AccessToken(SecretString::from(EXAMPLE_TOKEN));
        let hash = token.sha256();
        assert_compact_debug_snapshot!(hash.as_slice(), @"[11, 102, 58, 175, 81, 174, 38, 227, 173, 48, 158, 96, 20, 130, 99, 78, 7, 16, 241, 211, 195, 166, 110, 74, 193, 126, 53, 125, 42, 21, 23, 124]");
    }

    #[test]
    fn test_from_str() {
        let token = AccessToken::generate().finalize();
        let token = token.expose_secret();
        let token2 = assert_ok!(token.parse::<AccessToken>());
        assert_eq!(token2.finalize().expose_secret(), token);

        let str = "cio_tp_0000000000000000000000000000000w";
        assert_ok!(str.parse::<AccessToken>());

        let str = "invalid_token";
        assert_err_eq!(str.parse::<AccessToken>(), AccessTokenError::MissingPrefix);

        let str = "cio_tp_invalid_token";
        assert_err_eq!(str.parse::<AccessToken>(), AccessTokenError::InvalidLength);

        let str = "cio_tp_00000000000000000000000000";
        assert_err_eq!(str.parse::<AccessToken>(), AccessTokenError::InvalidLength);

        let str = "cio_tp_000000@0000000000000000000000000";
        assert_err_eq!(
            str.parse::<AccessToken>(),
            AccessTokenError::InvalidCharacter
        );

        let str = "cio_tp_00000000000000000000000000000000";
        assert_err_eq!(
            str.parse::<AccessToken>(),
            AccessTokenError::InvalidChecksum {
                claimed: '0',
                actual: 'w',
            }
        );
    }
}
