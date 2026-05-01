//! Dependency-injectable registry of [`OAuthProvider`] implementations.
//!
//! Constructed at app startup and attached to [`crate::app::App`]. The
//! session controller resolves providers by name; unknown names become a
//! 404 response.

use super::provider::OAuthProvider;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct ProviderRegistry {
    providers: HashMap<&'static str, Arc<dyn OAuthProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a provider to the registry. Panics on duplicate names.
    pub fn register(&mut self, provider: Arc<dyn OAuthProvider>) {
        let name = provider.name();
        assert!(
            !self.providers.contains_key(name),
            "provider already registered: {name}"
        );
        self.providers.insert(name, provider);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn OAuthProvider>> {
        self.providers.get(name).cloned()
    }

    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.providers.keys().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oauth::provider::MockOAuthProvider;

    fn mock_named(name: &'static str) -> Arc<dyn OAuthProvider> {
        let mut m = MockOAuthProvider::new();
        m.expect_name().return_const(name);
        Arc::new(m)
    }

    #[test]
    fn get_returns_registered_provider() {
        let mut r = ProviderRegistry::new();
        r.register(mock_named("github"));
        assert!(r.get("github").is_some());
    }

    #[test]
    fn get_returns_none_for_unknown() {
        let r = ProviderRegistry::new();
        assert!(r.get("bitbucket").is_none());
    }

    #[test]
    fn names_enumerates_registered_providers() {
        let mut r = ProviderRegistry::new();
        r.register(mock_named("github"));
        r.register(mock_named("bitbucket"));
        let mut names: Vec<_> = r.names().collect();
        names.sort();
        assert_eq!(names, vec!["bitbucket", "github"]);
    }

    #[test]
    #[should_panic(expected = "provider already registered: github")]
    fn double_register_panics() {
        let mut r = ProviderRegistry::new();
        r.register(mock_named("github"));
        r.register(mock_named("github"));
    }
}
