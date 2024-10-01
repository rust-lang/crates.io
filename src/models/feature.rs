use std::collections::BTreeMap;

pub type FeaturesMap = BTreeMap<String, Vec<String>>;

/// Splits the given [`FeaturesMap`] into two [`FeaturesMap`]s based on their
/// values.
///
/// See <https://rust-lang.github.io/rfcs/3143-cargo-weak-namespaced-features.html>.
pub fn split_features(features: FeaturesMap) -> (FeaturesMap, FeaturesMap) {
    features.into_iter().partition(|(_k, vals)| {
        !vals
            .iter()
            .any(|v| v.starts_with("dep:") || v.contains("?/"))
    })
}
