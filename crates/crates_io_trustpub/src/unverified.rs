use jsonwebtoken::errors::Error;
use jsonwebtoken::{DecodingKey, TokenData, Validation};
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::LazyLock;

/// [`Validation`] configuration for decoding JWTs without any
/// signature validation.
///
/// **This must only be used to extract the `iss` claim from the JWT, which
/// is then used to look up the corresponding OIDC key set.**
static NO_VALIDATION: LazyLock<Validation> = LazyLock::new(|| {
    let mut no_validation = Validation::default();
    no_validation.validate_aud = false;
    no_validation.validate_exp = false;
    no_validation.required_spec_claims = HashSet::new();
    no_validation.insecure_disable_signature_validation();
    no_validation
});

/// Empty [`DecodingKey`] used for decoding JWTs without any signature
/// validation.
///
/// **This must only be used to extract the `iss` claim from the JWT, which
/// is then used to look up the corresponding OIDC key set.**
static EMPTY_KEY: LazyLock<DecodingKey> = LazyLock::new(|| DecodingKey::from_secret(b""));

/// Claims that are extracted from the JWT without any signature
/// validation. Specifically, this only extracts the `iss` claim, which is
/// used to look up the corresponding OIDC key set to then verify the
/// JWT signature.
#[derive(Debug, Clone, Deserialize)]
pub struct UnverifiedClaims {
    pub iss: String,
}

impl UnverifiedClaims {
    /// Decode the JWT and extract the `iss` claim without any
    /// signature validation.
    ///
    /// **This must only be used to extract the `iss` claim from the JWT, which
    /// is then used to look up the corresponding OIDC key set.**
    pub fn decode(token: &str) -> Result<TokenData<Self>, Error> {
        jsonwebtoken::decode(token, &EMPTY_KEY, &NO_VALIDATION)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_keys::encode_for_testing;
    use claims::{assert_err, assert_ok, assert_some_eq};
    use insta::assert_compact_debug_snapshot;
    use jsonwebtoken::{EncodingKey, Header, encode};
    use serde::Serialize;

    #[derive(Debug, Serialize)]
    struct TestClaims {
        iss: String,
    }

    #[test]
    fn test_decode_valid_token() {
        const KEY_ID: &str = "test-key-id";
        const ISSUER: &str = "https://example.com";

        let header = Header {
            kid: Some(KEY_ID.to_string()),
            ..Default::default()
        };

        let iss = ISSUER.to_string();
        let claims = TestClaims { iss };

        let key = EncodingKey::from_secret(b"test-secret");
        let token = assert_ok!(encode(&header, &claims, &key));

        let decoded = assert_ok!(UnverifiedClaims::decode(&token));
        assert_some_eq!(decoded.header.kid, KEY_ID);
        assert_eq!(decoded.claims.iss, ISSUER);
    }

    #[test]
    fn test_decode_token_encoded_with_test_key() {
        const ISSUER: &str = "https://example.com";

        let iss = ISSUER.to_string();
        let claims = TestClaims { iss };
        let token = encode_for_testing(&claims).unwrap();

        let decoded = assert_ok!(UnverifiedClaims::decode(&token));
        assert_eq!(decoded.claims.iss, ISSUER);
    }

    #[test]
    fn test_decode_invalid_token() {
        let error = assert_err!(UnverifiedClaims::decode(""));
        assert_compact_debug_snapshot!(error, @"Error(InvalidToken)");

        let error = assert_err!(UnverifiedClaims::decode("invalid.token"));
        assert_compact_debug_snapshot!(error, @"Error(InvalidToken)");

        let error = assert_err!(UnverifiedClaims::decode("invalid.token.format"));
        assert_compact_debug_snapshot!(error, @"Error(Base64(InvalidLastSymbol(6, 100)))");
    }
}
