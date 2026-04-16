use aes_gcm::aead::{Aead, AeadCore, OsRng};
use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
use anyhow::{Context, Result};
use oauth2::AccessToken;

/// Deprecated: Use [OauthTokenEncryption] instead.
pub type GitHubTokenEncryption = OauthTokenEncryption;

/// A struct that encapsulates OAuth token encryption and decryption
/// using AES-256-GCM.
pub struct OauthTokenEncryption {
    cipher: Aes256Gcm,
}

impl std::fmt::Debug for OauthTokenEncryption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OauthTokenEncryption").finish()
    }
}

impl OauthTokenEncryption {
    /// Creates a new [OauthTokenEncryption] instance with the provided cipher
    pub fn new(cipher: Aes256Gcm) -> Self {
        Self { cipher }
    }

    /// Creates a new [OauthTokenEncryption] instance with a cipher for testing
    /// purposes.
    #[cfg(any(test, debug_assertions))]
    pub fn for_testing() -> Self {
        let test_key = b"test_key_32_bytes_long_for_tests";
        Self::new(Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(test_key)))
    }

    /// Creates a new [OauthTokenEncryption] instance from the environment
    ///
    /// Tries to read the `OAUTH_TOKEN_ENCRYPTION_KEY` environment variable first,
    /// which should be a 64-character hex string (32 bytes when decoded).
    /// Falls back to `GITHUB_TOKEN_ENCRYPTION_KEY` (deprecated) if the new
    /// variable is not set, emitting a warning when the fallback is used.
    pub fn from_environment() -> Result<Self> {
        let oauth_token_key = std::env::var("OAUTH_TOKEN_ENCRYPTION_KEY");
        let github_token_key = std::env::var("GITHUB_TOKEN_ENCRYPTION_KEY");

        let key_value = match (oauth_token_key, github_token_key) {
            (Ok(oauth_key), _) => oauth_key,
            (Err(_), Ok(github_key)) => {
                tracing::warn!(
                    "GITHUB_TOKEN_ENCRYPTION_KEY is deprecated; use OAUTH_TOKEN_ENCRYPTION_KEY instead"
                );
                github_key
            }
            (Err(_), Err(_)) => {
                anyhow::bail!(
                    "Either OAUTH_TOKEN_ENCRYPTION_KEY or GITHUB_TOKEN_ENCRYPTION_KEY environment variable must be set"
                );
            }
        };

        if key_value.len() != 64 {
            anyhow::bail!("Token encryption key must be exactly 64 hex characters");
        }

        let key_bytes = hex::decode(key_value.as_bytes())
            .context("Token encryption key must be exactly 64 hex characters")?;

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));

        Ok(Self::new(cipher))
    }

    /// Encrypts an OAuth access token using AES-256-GCM
    ///
    /// The encrypted data format is: `[12-byte nonce][encrypted data]`
    /// The nonce is randomly generated for each encryption to ensure uniqueness.
    pub fn encrypt(&self, plaintext: &str) -> Result<Vec<u8>> {
        // Generate a random nonce for this encryption
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        // Encrypt the token
        let encrypted = self
            .cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|error| anyhow::anyhow!("Failed to encrypt token: {error}"))?;

        // Combine nonce + ciphertext (which includes the auth tag)
        let mut result = Vec::with_capacity(nonce.len() + encrypted.len());
        result.extend_from_slice(&nonce);
        result.extend_from_slice(&encrypted);

        Ok(result)
    }

    /// Decrypts an OAuth access token using AES-256-GCM
    ///
    /// Expects the data format: `[12-byte nonce][encrypted data]`
    pub fn decrypt(&self, encrypted: &[u8]) -> Result<AccessToken> {
        if encrypted.len() < 12 {
            anyhow::bail!("Invalid encrypted token: too short");
        }

        // Extract nonce and ciphertext
        let (nonce_bytes, ciphertext) = encrypted.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt the token
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .context("Failed to decrypt token")?;

        let plaintext =
            String::from_utf8(plaintext).context("Decrypted token is not valid UTF-8")?;

        Ok(AccessToken::new(plaintext))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes_gcm::{Key, KeyInit};
    use claims::{assert_err, assert_ok};
    use insta::assert_snapshot;

    fn create_test_encryption() -> OauthTokenEncryption {
        let key = Key::<Aes256Gcm>::from_slice(b"test_master_key_32_bytes_long!!!");
        let cipher = Aes256Gcm::new(key);
        OauthTokenEncryption { cipher }
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let encryption = create_test_encryption();
        let original_token = "ghs_test_token_123456789";

        // Encrypt the token
        let encrypted = assert_ok!(encryption.encrypt(original_token));

        // Decrypt it back
        let decrypted = assert_ok!(encryption.decrypt(&encrypted));

        assert_eq!(original_token, decrypted.secret());
    }

    #[test]
    fn test_encrypt_produces_different_ciphertext() {
        let encryption = create_test_encryption();
        let token = "ghs_test_token_123456789";

        // Encrypt the same token twice
        let encrypted1 = assert_ok!(encryption.encrypt(token));
        let encrypted2 = assert_ok!(encryption.encrypt(token));

        // Should produce different ciphertext due to random nonce
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to the same plaintext
        let decrypted1 = assert_ok!(encryption.decrypt(&encrypted1));
        let decrypted2 = assert_ok!(encryption.decrypt(&encrypted2));

        assert_eq!(decrypted1.secret(), decrypted2.secret());
        assert_eq!(decrypted1.secret(), token);
    }

    #[test]
    fn test_invalid_encrypted_data() {
        let encryption = create_test_encryption();

        // Too short
        let err = assert_err!(encryption.decrypt(&[1, 2, 3]));
        assert_snapshot!(err, @"Invalid encrypted token: too short");

        // Invalid data
        let invalid_data = vec![0u8; 50];
        let err = assert_err!(encryption.decrypt(&invalid_data));
        assert_snapshot!(err, @"Failed to decrypt token");
    }

    #[test]
    fn test_different_keys() {
        let encryption1 = create_test_encryption();

        // Create a different encryption with a different key
        let key2 = Key::<Aes256Gcm>::from_slice(b"different_key_32_bytes_long!!!!!");
        let cipher2 = Aes256Gcm::new(key2);
        let encryption2 = OauthTokenEncryption { cipher: cipher2 };

        let token = "ghs_test_token_123456789";

        // Encrypt with encryption1
        let encrypted = assert_ok!(encryption1.encrypt(token));

        // Try to decrypt with encryption2 (should fail)
        let err = assert_err!(encryption2.decrypt(&encrypted));
        assert_snapshot!(err, @"Failed to decrypt token");

        // But encryption1 should still work
        let decrypted = assert_ok!(encryption1.decrypt(&encrypted));
        assert_eq!(decrypted.secret(), token);
    }

    #[test]
    fn prefers_new_env_var_when_both_set() {
        // Test that we read from OAUTH_TOKEN_ENCRYPTION_KEY when both are set
        let new_key = "0af877502cf11413eaa64af985fe1f8ed250ac9168a3b2db7da52cd5cc6116a9";
        let old_key = "1bf877502cf11413eaa64af985fe1f8ed250ac9168a3b2db7da52cd5cc6116a9";

        // Set both env vars
        unsafe {
            std::env::set_var("OAUTH_TOKEN_ENCRYPTION_KEY", new_key);
            std::env::set_var("GITHUB_TOKEN_ENCRYPTION_KEY", old_key);
        }

        let result = OauthTokenEncryption::from_environment();
        assert_ok!(result, "Should succeed with new env var");

        // Clean up
        unsafe {
            std::env::remove_var("OAUTH_TOKEN_ENCRYPTION_KEY");
            std::env::remove_var("GITHUB_TOKEN_ENCRYPTION_KEY");
        }
    }

    #[test]
    fn falls_back_to_legacy_env_var() {
        // Test that we fall back to GITHUB_TOKEN_ENCRYPTION_KEY when new one is absent
        let old_key = "0af877502cf11413eaa64af985fe1f8ed250ac9168a3b2db7da52cd5cc6116a9";

        // Clear new var, set old one
        unsafe {
            std::env::remove_var("OAUTH_TOKEN_ENCRYPTION_KEY");
            std::env::set_var("GITHUB_TOKEN_ENCRYPTION_KEY", old_key);
        }

        let result = OauthTokenEncryption::from_environment();
        assert_ok!(result, "Should fall back to legacy key");

        // Clean up
        unsafe {
            std::env::remove_var("GITHUB_TOKEN_ENCRYPTION_KEY");
        }
    }

    #[test]
    fn errors_when_both_absent() {
        // Test that we error when neither env var is set
        unsafe {
            std::env::remove_var("OAUTH_TOKEN_ENCRYPTION_KEY");
            std::env::remove_var("GITHUB_TOKEN_ENCRYPTION_KEY");
        }

        let result = OauthTokenEncryption::from_environment();
        assert_err!(result, "Should error when no key is present");
    }

    #[test]
    fn errors_on_invalid_hex() {
        // Test that we error on invalid hex input
        unsafe {
            std::env::set_var("OAUTH_TOKEN_ENCRYPTION_KEY", "not_64_hex_chars_at_all");
            std::env::remove_var("GITHUB_TOKEN_ENCRYPTION_KEY");
        }

        let result = OauthTokenEncryption::from_environment();
        assert_err!(result, "Should error on invalid hex");

        // Clean up
        unsafe {
            std::env::remove_var("OAUTH_TOKEN_ENCRYPTION_KEY");
        }
    }

    #[test]
    fn errors_on_wrong_length() {
        // Test that we error when key is not exactly 64 hex characters
        unsafe {
            std::env::set_var("OAUTH_TOKEN_ENCRYPTION_KEY", "deadbeef");
            std::env::remove_var("GITHUB_TOKEN_ENCRYPTION_KEY");
        }

        let result = OauthTokenEncryption::from_environment();
        assert_err!(result, "Should error when key is wrong length");

        // Clean up
        unsafe {
            std::env::remove_var("OAUTH_TOKEN_ENCRYPTION_KEY");
        }
    }

    #[test]
    fn debug_impl_does_not_leak_key() {
        let enc = OauthTokenEncryption::for_testing();
        let debug = format!("{enc:?}");
        assert!(debug.contains("OauthTokenEncryption"), "got: {debug}");
        // Verify the key material isn't in the debug output
        assert!(!debug.contains("test_key"), "key leaked in debug: {debug}");
    }

    #[test]
    fn for_testing_produces_working_instance() {
        let enc = OauthTokenEncryption::for_testing();
        let encrypted = assert_ok!(enc.encrypt("hello"));
        let decrypted = assert_ok!(enc.decrypt(&encrypted));
        assert_eq!(decrypted.secret(), "hello");
    }
}
