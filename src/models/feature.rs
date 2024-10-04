use std::collections::BTreeMap;

pub type FeaturesMap = BTreeMap<String, Vec<String>>;

/// Splits the given [`FeaturesMap`] into two [`FeaturesMap`]s based on their
/// values.
///
/// See <https://rust-lang.github.io/rfcs/3143-cargo-weak-namespaced-features.html>.
pub fn split_features(features: FeaturesMap) -> (FeaturesMap, FeaturesMap) {
    const ITERATION_LIMIT: usize = 100;

    // First, we partition the features into two groups: those that use the new
    // features syntax (`features2`) and those that don't (`features`).
    let (mut features, mut features2) =
        features
            .into_iter()
            .partition::<FeaturesMap, _>(|(_k, vals)| {
                !vals
                    .iter()
                    .any(|v| v.starts_with("dep:") || v.contains("?/"))
            });

    // Then, we recursively move features from `features` to `features2` if they
    // depend on features in `features2`.
    for i in (0..ITERATION_LIMIT).rev() {
        let split = features
            .into_iter()
            .partition::<FeaturesMap, _>(|(_k, vals)| {
                !vals.iter().any(|v| features2.contains_key(v))
            });

        features = split.0;

        if !split.1.is_empty() {
            features2.extend(split.1);

            if i == 0 {
                warn!("Iteration limit reached while splitting features!");
            }
        } else {
            break;
        }
    }

    (features, features2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::{assert_compact_debug_snapshot, assert_debug_snapshot};

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

        assert_compact_debug_snapshot!(features, @"{}");
        assert_compact_debug_snapshot!(features2, @r#"{"feature1": ["feature2"], "feature2": ["feature3"], "feature3": ["dep:foo"]}"#);
    }

    #[test]
    fn test_split_features_clap() {
        let json = json!({
            "env": ["clap_builder/env"],
            "std": ["clap_builder/std"],
            "help": ["clap_builder/help"],
            "cargo": ["clap_builder/cargo"],
            "color": ["clap_builder/color"],
            "debug": ["clap_builder/debug", "clap_derive?/debug"],
            "usage": ["clap_builder/usage"],
            "derive": ["dep:clap_derive"],
            "string": ["clap_builder/string"],
            "default": ["std", "color", "help", "usage", "error-context", "suggestions"],
            "unicode": ["clap_builder/unicode"],
            "wrap_help": ["clap_builder/wrap_help"],
            "deprecated": ["clap_builder/deprecated", "clap_derive?/deprecated"],
            "suggestions": ["clap_builder/suggestions"],
            "unstable-v5": ["clap_builder/unstable-v5", "clap_derive?/unstable-v5", "deprecated"],
            "unstable-doc": ["clap_builder/unstable-doc", "derive"],
            "unstable-ext": ["clap_builder/unstable-ext"],
            "error-context": ["clap_builder/error-context"],
            "unstable-styles": ["clap_builder/unstable-styles"]
        });

        let features = serde_json::from_value::<FeaturesMap>(json).unwrap();
        assert_debug_snapshot!(split_features(features));
    }
}
