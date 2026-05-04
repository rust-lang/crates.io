#![doc = include_str!("../README.md")]

pub mod access_token;
pub mod github;
pub mod gitlab;
pub mod keystore;
#[cfg(any(test, feature = "test-helpers"))]
pub mod test_keys;
pub mod unverified;

/// Leeway applied to JWT `exp`, `nbf`, and `iat` validation to account for
/// clock skew between the OIDC issuer and crates.io.
///
/// The same value is added to the `expires_at` of stored JTIs in the
/// `trustpub_used_jtis` table so that replay protection covers the full
/// window during which a JWT's signature is still accepted by
/// `jsonwebtoken::decode`. Keeping these two values in lock-step is a
/// security invariant: if the JTI record expires before the JWT signature
/// does, an attacker who has obtained the JWT can replay it to mint a fresh
/// upload token.
pub const JWT_LEEWAY: chrono::Duration = chrono::Duration::seconds(60);
