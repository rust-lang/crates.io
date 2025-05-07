mod r#impl;
mod load_jwks;

use async_trait::async_trait;
pub use r#impl::RealOidcKeyStore;
use jsonwebtoken::DecodingKey;

/// A trait for fetching OIDC keys from a key store.
///
/// The main implementation is [`RealOidcKeyStore`], but for testing purposes
/// there is also a mock implementation available.
#[cfg_attr(feature = "test-helpers", mockall::automock)]
#[async_trait]
pub trait OidcKeyStore: Send + Sync {
    /// Fetches a [`DecodingKey`] from the key store using the provided `key_id`.
    ///
    /// If the key is not found on the server, it will return `None`. If there
    /// is an error while fetching the key, it will return an error.
    async fn get_oidc_key(&self, key_id: &str) -> anyhow::Result<Option<DecodingKey>>;
}
