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

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_compact_debug_snapshot;

    #[test]
    fn test_split_features_no_deps() {
        let mut features = FeaturesMap::new();
        features.insert(
            "feature1".to_string(),
            vec!["val1".to_string(), "val2".to_string()],
        );
        features.insert("feature2".to_string(), vec!["val3".to_string()]);

        let (features, features2) = split_features(features);

        assert_compact_debug_snapshot!(features, @r#"{"feature1": ["val1", "val2"], "feature2": ["val3"]}"#);
        assert_compact_debug_snapshot!(features2, @"{}");
    }

    #[test]
    fn test_split_features_with_deps() {
        let mut features = FeaturesMap::new();
        features.insert(
            "feature1".to_string(),
            vec!["dep:val1".to_string(), "val2".to_string()],
        );
        features.insert(
            "feature2".to_string(),
            vec!["val3".to_string(), "val4?/val5".to_string()],
        );

        let (features, features2) = split_features(features);

        assert_compact_debug_snapshot!(features, @"{}");
        assert_compact_debug_snapshot!(features2, @r#"{"feature1": ["dep:val1", "val2"], "feature2": ["val3", "val4?/val5"]}"#);
    }

    #[test]
    fn test_split_features_mixed() {
        let mut features = FeaturesMap::new();
        features.insert(
            "feature1".to_string(),
            vec!["val1".to_string(), "val2".to_string()],
        );
        features.insert("feature2".to_string(), vec!["dep:val3".to_string()]);
        features.insert(
            "feature3".to_string(),
            vec!["val4".to_string(), "val5?/val6".to_string()],
        );

        let (features, features2) = split_features(features);

        assert_compact_debug_snapshot!(features, @r#"{"feature1": ["val1", "val2"]}"#);
        assert_compact_debug_snapshot!(features2, @r#"{"feature2": ["dep:val3"], "feature3": ["val4", "val5?/val6"]}"#);
    }

    #[test]
    fn test_split_features_nested() {
        let mut features = FeaturesMap::new();
        features.insert("feature1".to_string(), vec!["feature2".to_string()]);
        features.insert("feature2".to_string(), vec![]);
        features.insert("feature3".to_string(), vec!["feature1".to_string()]);

        let (features, features2) = split_features(features);

        assert_compact_debug_snapshot!(features, @r#"{"feature1": ["feature2"], "feature2": [], "feature3": ["feature1"]}"#);
        assert_compact_debug_snapshot!(features2, @"{}");
    }

    #[test]
    fn test_split_features_nested_mixed() {
        let mut features = FeaturesMap::new();
        features.insert("feature1".to_string(), vec!["feature2".to_string()]);
        features.insert("feature2".to_string(), vec!["feature3".to_string()]);
        features.insert("feature3".to_string(), vec!["dep:foo".to_string()]);

        let (features, features2) = split_features(features);

        assert_compact_debug_snapshot!(features, @r#"{"feature1": ["feature2"], "feature2": ["feature3"]}"#);
        assert_compact_debug_snapshot!(features2, @r#"{"feature3": ["dep:foo"]}"#);
    }
}
