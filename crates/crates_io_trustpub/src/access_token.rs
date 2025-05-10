use secrecy::{ExposeSecret, SecretString};
use sha2::digest::Output;
use sha2::{Digest, Sha256};

/// A temporary access token used to publish crates to crates.io using
/// the "Trusted Publishing" feature.
#[derive(Debug)]
pub struct AccessToken(SecretString);

impl AccessToken {
    const PREFIX: &str = "cio_tp_";

    /// Generate a new access token.
    pub fn generate() -> Self {
        Self::from_u64s(rand::random(), rand::random())
    }

    /// Create an access token from two u64 values.
    ///
    /// This is used internally by the `generate()` fn and is extracted
    /// to a separate function for testing purposes.
    fn from_u64s(r1: u64, r2: u64) -> Self {
        let plaintext = format!("{}{r1:016x}{r2:016x}", Self::PREFIX);
        Self(SecretString::from(plaintext))
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let str = String::from_utf8(bytes.into()).ok()?;

        let suffix = str.strip_prefix(Self::PREFIX)?;
        if suffix.len() != 32 {
            return None;
        }

        let is_hexdigit = |c| matches!(c, 'a'..='f') || c.is_ascii_digit();
        if !suffix.chars().all(is_hexdigit) {
            return None;
        }

        Some(Self(SecretString::from(str)))
    }

    /// Generate a SHA256 hash of the access token.
    pub fn sha256(&self) -> Output<Sha256> {
        Sha256::digest(self.0.expose_secret())
    }
}

impl ExposeSecret<str> for AccessToken {
    fn expose_secret(&self) -> &str {
        self.0.expose_secret()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some};
    use insta::assert_snapshot;

    #[test]
    fn test_generate() {
        let token = AccessToken::generate();
        let token_str = token.expose_secret();
        assert!(token_str.starts_with(AccessToken::PREFIX));
        assert_eq!(token_str.len(), AccessToken::PREFIX.len() + 32);
    }

    #[test]
    fn test_serialization() {
        let token = AccessToken::from_u64s(0, 0);
        assert_snapshot!(token.expose_secret(), @"cio_tp_00000000000000000000000000000000");

        let token = AccessToken::from_u64s(u64::MAX, u64::MAX);
        assert_snapshot!(token.expose_secret(), @"cio_tp_ffffffffffffffffffffffffffffffff");

        let token = AccessToken::from_u64s(0xc0ffee, 0xfa8072);
        assert_snapshot!(token.expose_secret(), @"cio_tp_0000000000c0ffee0000000000fa8072");
    }

    #[test]
    fn test_sha256() {
        let token = AccessToken::generate();
        let hash = token.sha256();
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_from_bytes() {
        let token = AccessToken::generate();
        let bytes = token.expose_secret().as_bytes();
        let token2 = assert_some!(AccessToken::from_bytes(bytes));
        assert_eq!(token.expose_secret(), token2.expose_secret());

        let bytes = b"cio_tp_00000000000000000000000000000000";
        assert_some!(AccessToken::from_bytes(bytes));

        let invalid_bytes = b"invalid_token";
        assert_none!(AccessToken::from_bytes(invalid_bytes));

        let invalid_bytes = b"cio_tp_invalid_token";
        assert_none!(AccessToken::from_bytes(invalid_bytes));

        let invalid_bytes = b"cio_tp_00000000000000000000000000";
        assert_none!(AccessToken::from_bytes(invalid_bytes));

        let invalid_bytes = b"cio_tp_000000x0000000000000000000000000";
        assert_none!(AccessToken::from_bytes(invalid_bytes));
    }
}
