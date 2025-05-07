use super::OidcKeyStore;
use super::load_jwks::load_jwks;
use async_trait::async_trait;
use jsonwebtoken::DecodingKey;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// The main implementation of the [`OidcKeyStore`] trait.
///
/// This struct fetches OIDC keys from a remote provider and caches them. If
/// a key is not found in the cache, it will attempt to refresh the cached
/// key set, unless the cache has just recently been refreshed.
pub struct RealOidcKeyStore {
    issuer_uri: String,
    client: reqwest::Client,
    cache: RwLock<Cache>,
}

#[derive(Default)]
struct Cache {
    keys: HashMap<String, DecodingKey>,
    last_update: Option<Instant>,
}

impl RealOidcKeyStore {
    /// Creates a new instance of [`RealOidcKeyStore`].
    pub fn new(issuer_uri: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("crates.io")
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        Self {
            issuer_uri,
            client,
            cache: RwLock::new(Cache::default()),
        }
    }
}

#[async_trait]
impl OidcKeyStore for RealOidcKeyStore {
    async fn get_oidc_key(&self, key_id: &str) -> anyhow::Result<Option<DecodingKey>> {
        const MIN_AGE_BEFORE_REFRESH: Duration = Duration::from_secs(60);

        // First, try to get the key with just a read lock.
        let cache = self.cache.read().await;
        if let Some(key) = cache.keys.get(key_id) {
            return Ok(Some(key.clone()));
        }

        // If that fails, drop the read lock before acquiring the write lock.
        drop(cache);

        let mut cache = self.cache.write().await;
        if cache
            .last_update
            .is_some_and(|last_update| last_update.elapsed() < MIN_AGE_BEFORE_REFRESH)
        {
            // If we're in a cooldown from a previous refresh, return
            // whatever is in the cache.
            return Ok(cache.keys.get(key_id).cloned());
        }

        // Load the keys from the OIDC provider.
        let jwks = load_jwks(&self.client, &self.issuer_uri).await?;

        cache.keys.clear();
        for key in jwks.keys {
            if let Some(key_id) = &key.common.key_id {
                let decoding_key = DecodingKey::from_jwk(&key)?;
                cache.keys.insert(key_id.clone(), decoding_key);
            }
        }

        cache.last_update = Some(Instant::now());

        Ok(cache.keys.get(key_id).cloned())
    }
}
