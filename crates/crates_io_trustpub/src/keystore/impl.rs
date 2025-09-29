use super::OidcKeyStore;
use super::load_jwks::load_jwks;
use async_trait::async_trait;
use jsonwebtoken::DecodingKey;
use jsonwebtoken::jwk::JwkSet;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::warn;

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

impl Cache {
    /// Returns true if the cache was updated within the minimum refresh interval.
    fn recently_updated(&self) -> bool {
        const MIN_AGE_BEFORE_REFRESH: Duration = Duration::from_secs(60);

        self.last_update
            .is_some_and(|last_update| last_update.elapsed() < MIN_AGE_BEFORE_REFRESH)
    }

    /// Updates the key cache with a new JWK Set, replacing all existing keys.
    ///
    /// This method clears the current cache and populates it with decoding keys
    /// from the provided JWK Set. Keys without a key ID are skipped with a warning.
    /// The cache's last update timestamp is set to the current time.
    fn update(&mut self, jwks: &JwkSet) -> anyhow::Result<()> {
        self.keys.clear();
        for key in &jwks.keys {
            if let Some(key_id) = &key.common.key_id {
                let decoding_key = DecodingKey::from_jwk(key)?;
                self.keys.insert(key_id.clone(), decoding_key);
            } else {
                warn!("OIDC key without a key ID found, skipping.");
            }
        }

        self.last_update = Some(Instant::now());

        Ok(())
    }
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
        // First, try to get the key with just a read lock.
        let cache = self.cache.read().await;
        if let Some(key) = cache.keys.get(key_id) {
            return Ok(Some(key.clone()));
        }

        // If that fails, drop the read lock before acquiring the write lock.
        drop(cache);

        let mut cache = self.cache.write().await;
        if cache.recently_updated() {
            // If we're in a cooldown from a previous refresh, return
            // whatever is in the cache, which will probably be None
            // given the previous check under the read lock.
            return Ok(cache.keys.get(key_id).cloned());
        }

        // Load the keys from the OIDC provider.
        let jwks = load_jwks(&self.client, &self.issuer_uri).await?;
        cache.update(&jwks)?;

        Ok(cache.keys.get(key_id).cloned())
    }
}
